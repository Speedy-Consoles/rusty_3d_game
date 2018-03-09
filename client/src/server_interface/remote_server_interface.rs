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
use shared::net::DisconnectReason;
use shared::net::MAX_MESSAGE_LENGTH;
use shared::consts;
use shared::consts::TICK_SPEED;
use shared::consts::NEWEST_TICK_TIME_WEIGHT;
use shared::util;

use super::ConnectionState;
use super::ServerInterface;
use super::TickInfo;
use self::InternalState::*;
use self::SnapshotState::*;

enum SnapshotState {
    BeforeSnapshot,
    AfterSnapshot {
        start_tick_time: Instant,
        last_snapshot: Snapshot, // TODO store multiple snapshots so we can catch up without predicting
        tick_info: Option<TickInfo>, // None until first tick after snapshot
    }
}

enum InternalState {
    Connecting,
    Connected {
        my_player_id: u64,
        snapshot_state: SnapshotState,
    },
    Disconnecting,
    Disconnected,
}

pub struct RemoteServerInterface {
    socket: UdpSocket,
    internal_state: InternalState,
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
                sent_inputs: HashMap::new(),
            };
            rsi.send(ClientMessage::ConnectionRequest);
            Ok(rsi)
        })
    }

    fn on_snapshot(&mut self, snapshot: Snapshot) {
        let recv_time = Instant::now();
        let new_start_tick_time = recv_time - util::mult_duration(
            consts::tick_duration(),
            snapshot.get_tick()
        ) + consts::tick_time_tolerance();
        // this adds some extra delay in our ticks to make it likely
        // that the next snapshot will be there on any tick
        if let Connected { ref mut snapshot_state, .. } = self.internal_state {
            match snapshot_state {
                &mut BeforeSnapshot => *snapshot_state = AfterSnapshot {
                    start_tick_time: new_start_tick_time,
                    tick_info: None,
                    last_snapshot: snapshot,
                },
                &mut AfterSnapshot { ref mut start_tick_time, ref mut last_snapshot, .. } => {
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
    }

    fn handle_message(&mut self, msg: ServerMessage) {
        use shared::net::ServerMessage::*;
        match msg {
            ConnectionConfirm(my_player_id) => self.internal_state = Connected {
                my_player_id,
                snapshot_state: BeforeSnapshot,
            },
            Snapshot(s) => self.on_snapshot(s),
            PlayerDisconnect { id, name, reason } => {
                match self.internal_state {
                    Connected { my_player_id, .. } if my_player_id == id => {
                        if let DisconnectReason::Kicked = reason {
                             println!("You were kicked.");
                        }
                        self.internal_state = Disconnected;
                    },
                    Connected { .. } => {
                        let reason_str = match reason {
                            DisconnectReason::Disconnected => "left",
                            DisconnectReason::TimedOut => "timed out",
                            DisconnectReason::Kicked => "was kicked",
                        };
                        println!("{} {}.", name, reason_str);
                    },
                    _ => ()
                }
            },
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
    fn tick(&mut self, model: &mut Model, input: CharacterInput) {
        let tick_lag = 20;// TODO use adaptive delay and prevent predicted tick decreasing
        if let Connected {
            snapshot_state: AfterSnapshot { start_tick_time, ref mut tick_info, .. },
            ..
        } = self.internal_state {
            let now = Instant::now();

            let tick;
            let next_tick_time;
            if let &mut Some(ref tick_info) = tick_info {
                let target_float_tick = util::elapsed_ticks_float(
                    tick_info.next_tick_time - start_tick_time,
                    TICK_SPEED
                );
                let tick_time = tick_info.next_tick_time;
                let float_tick = (tick_info.tick + 1) as f64;
                let float_tick_diff = target_float_tick - float_tick;
                let param1 = TICK_SPEED as f64 / 4.0;
                let param2 = TICK_SPEED as f64 / 4.0;
                let param3 = 0.2;
                if float_tick_diff < 0.0 {
                    tick = tick_info.tick + 1;
                    next_tick_time = tick_time + util::mult_duration_float(
                        consts::tick_duration(),
                        (-float_tick_diff / param1 + 1.0).min(2.0),
                    );
                } else if float_tick_diff <= param2 {
                    tick = tick_info.tick + 1;
                    next_tick_time = tick_time + util::mult_duration_float(
                        consts::tick_duration(),
                        1.0 - float_tick_diff / param2 / param3,
                    );
                } else {
                    println!("SKIPPING!");
                    tick = target_float_tick as u64;
                    next_tick_time = tick_info.next_tick_time + consts::tick_duration();
                }
            } else {
                tick = util::elapsed_ticks(now - start_tick_time, TICK_SPEED);
                next_tick_time = now + consts::tick_duration();
            }

            //println!("{}", tick);

            *tick_info = Some(TickInfo {
                tick,
                predicted_tick: tick + tick_lag,
                next_tick_time,
            });
        }

        if let Connected {
            snapshot_state: AfterSnapshot { ref last_snapshot, tick_info: Some(ref tick_info), .. },
            ..
        } = self.internal_state {
            // send input
            assert!(tick_info.predicted_tick > last_snapshot.get_tick());
            let input_tick = tick_info.predicted_tick;
            let msg = ClientMessage::Input { tick: input_tick, input };
            self.send(msg);
            self.sent_inputs.insert(input_tick, input);

            // update model
            *model = last_snapshot.get_model().clone(); // TODO do this better
            for _ in last_snapshot.get_tick()..tick_info.tick {
                model.tick();
            }

            if tick_info.tick > last_snapshot.get_tick() {
                println!("WARNING: {} ticks ahead of last snapshot!",
                         tick_info.tick - last_snapshot.get_tick());
            } else if tick_info.tick < last_snapshot.get_tick() {
                println!("WARNING: {} ticks behind last snapshot!",
                         last_snapshot.get_tick() - tick_info.tick);
            }
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

    fn get_connection_state(&self) -> ConnectionState {
        match self.internal_state {
            Connecting
            | Connected { snapshot_state: BeforeSnapshot, .. }
            | Connected { snapshot_state: AfterSnapshot { tick_info: None, .. }, .. }
                => ConnectionState::Connecting,
            Disconnecting => ConnectionState::Disconnecting,
            Disconnected => ConnectionState::Disconnected,
            Connected {
                my_player_id,
                snapshot_state: AfterSnapshot { tick_info: Some(tick_info), .. }
            } => ConnectionState::Connected { my_player_id, tick_info }
        }
    }

    fn get_character_input(&self, tick: u64) -> Option<CharacterInput> {
        self.sent_inputs.get(&tick).map(|input| *input)
    }

    fn disconnect(&mut self) {
        self.send(ClientMessage::Leave);
        self.internal_state = Disconnecting;
    }
}