use std::time::Instant;
use std::io;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::io::ErrorKind;
use std::collections::HashMap;

use shared::model::Model;
use shared::model::world::character::CharacterInput;
use shared::net::ServerMessage;
use shared::net::ClientMessage;
use shared::net::Packable;
use shared::net::Snapshot;
use shared::net::MAX_MESSAGE_LENGTH;
use shared::consts;
use shared::consts::TICK_SPEED;
use shared::consts::NEWEST_TICK_TIME_WEIGHT;
use shared::util;

use super::ConnectionState;
use super::ServerInterface;
use super::TickInfo;
use self::InternalState::*;

enum InternalState {
    Connecting,
    BeforeSnapshot { my_player_id: u64 },
    AfterSnapshot {
        my_player_id: u64,
        start_tick_time: Instant,
        last_snapshot: Snapshot,
    },
    Disconnecting,
    Disconnected,
}

pub struct RemoteServerInterface {
    socket: UdpSocket,
    internal_state: InternalState,
    tick_info: Option<TickInfo>,
    tick_lag: u64,
    sent_inputs: HashMap<u64, CharacterInput>,
}

impl RemoteServerInterface {
    pub fn new(addr: SocketAddr) -> io::Result<RemoteServerInterface> {
        // let the os decide over port
        let local_addr = match addr {
            SocketAddr::V4(_) => "0.0.0.0:0",
            SocketAddr::V6(_) => "[::]:0",
        };
        UdpSocket::bind(local_addr).and_then(|socket| {
            if let Err(e) = socket.connect(addr) {
                return Err(e);
            }
            let rsi = RemoteServerInterface {
                socket,
                internal_state: Connecting,
                tick_info: None,
                tick_lag: 0,
                sent_inputs: HashMap::new(),
            };
            rsi.send(ClientMessage::ConnectionRequest);
            Ok(rsi)
        })
    }

    fn on_snapshot(&mut self, snapshot: Snapshot) {
        let new_start_tick_time = Instant::now() - util::mult_duration(
            consts::tick_interval(),
            snapshot.get_tick()
        ) + consts::tick_time_tolerance();
        // this adds some extra delay in our ticks to make it likely
        // that the next snapshot will be there on any tick
        match self.internal_state {
            Connecting | Disconnecting | Disconnected => (), // ignore snapshot
            BeforeSnapshot { my_player_id } => self.internal_state = AfterSnapshot {
                my_player_id,
                start_tick_time: new_start_tick_time,
                last_snapshot: snapshot,
            },
            AfterSnapshot { ref mut start_tick_time, ref mut last_snapshot, .. } => {
                if snapshot > *last_snapshot {
                    for tick in (last_snapshot.get_tick() + 1)..(snapshot.get_tick() + 1) {
                        self.sent_inputs.remove(&tick);
                    }
                    *start_tick_time = util::mix_time(
                        *start_tick_time,
                        new_start_tick_time,
                        NEWEST_TICK_TIME_WEIGHT
                    );
                    *last_snapshot = snapshot;
                }
            },
        }
    }

    fn handle_message(&mut self, msg: ServerMessage) {
        use shared::net::ServerMessage::*;
        match msg {
            ConnectionConfirm(my_player_id) => self.internal_state = BeforeSnapshot {
                my_player_id
            },
            Snapshot(s) => self.on_snapshot(s),
            Kick => self.internal_state = Disconnected,
            EchoResponse(_) => (),
        }
    }

    fn send(&self, msg: ClientMessage) {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        let amount = msg.pack(&mut buf).unwrap();
        self.socket.send(&buf[..amount]).unwrap();
    }

    fn recv(&self) -> Option<ServerMessage> {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        match self.socket.recv(&mut buf) {
            Ok(amount) => {
                match ServerMessage::unpack(&buf[..amount]) {
                    Ok(msg) => Some(msg),
                    Err(e) => {
                        println!("{:?}", e);
                        None
                    },
                }
            },
            Err(e) => {
                match e.kind() {
                    ErrorKind::WouldBlock | ErrorKind::TimedOut => (),
                    _ => println!("{:?}", e),
                };
                None
            }
        }
    }
}

impl ServerInterface for RemoteServerInterface {
    fn tick(&mut self, model: &mut Model, input: CharacterInput) -> Instant {
        if let AfterSnapshot { start_tick_time, ref last_snapshot, .. } = self.internal_state {
            // update tick info
            // TODO think of a better way to assure smooth time
            let diff = Instant::now() - start_tick_time;
            let tick = util::elapsed_ticks(diff, TICK_SPEED);
            let tick_time = start_tick_time + util::mult_duration(consts::tick_interval(), tick);
            let overwrite = if let Some(ref tick_info) = self.tick_info {
                tick_info.tick < tick
            } else {
                true
            };
            if overwrite {
                self.tick_info = Some(TickInfo {
                    tick,
                    tick_time,
                });
            }

            // send input
            self.tick_lag = 50; // TODO use realistic delay
            let input_tick = tick + self.tick_lag;
            let msg = ClientMessage::Input { tick: input_tick, input };
            self.send(msg);
            self.sent_inputs.insert(input_tick, input);

            // update model
            *model = last_snapshot.get_model().clone(); // TODO do this better
            for _ in last_snapshot.get_tick()..tick {
                model.tick();
            }

            tick_time + consts::tick_interval()
        } else {
            Instant::now() + consts::tick_interval()
        }
    }

    fn handle_traffic(&mut self, until: Instant) {
        loop {
            let now = Instant::now();
            if until <= now {
                break;
            }
            self.socket.set_read_timeout(Some(until - now)).unwrap();
            if let Some(msg) = self.recv() {
                self.handle_message(msg);
            }
        }
    }

    fn get_tick_info(&self) -> Option<TickInfo> {
        self.tick_info
    }

    fn get_tick_lag(&self) -> u64 {
        self.tick_lag
    }

    fn get_my_player_id(&self) -> Option<u64> {
        match self.internal_state {
            BeforeSnapshot { my_player_id }
            | AfterSnapshot { my_player_id, .. } => Some(my_player_id),
            _ => None
        }
    }

    fn get_character_input(&self, tick: u64) -> Option<CharacterInput> {
        self.sent_inputs.get(&tick).map(|input| *input)
    }

    fn get_connection_state(&self) -> ConnectionState {
        match self.internal_state {
            Connecting => ConnectionState::Connecting,
            BeforeSnapshot {..} | AfterSnapshot {..} => ConnectionState::Connected,
            Disconnecting => ConnectionState::Disconnecting,
            Disconnected => ConnectionState::Disconnected,
        }
    }

    fn disconnect(&mut self) {
        self.send(ClientMessage::Leave);
        self.internal_state = Disconnecting;
    }
}