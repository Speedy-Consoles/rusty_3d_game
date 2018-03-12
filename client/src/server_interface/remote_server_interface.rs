use std::time::Instant;
use std::time::Duration;
use std::io;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::io::ErrorKind;
use std::collections::HashMap;
use std::iter;
use std::ops::Sub;
use std::ops::Add;

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
use shared::consts::NEWEST_START_TICK_TIME_WEIGHT;
use shared::consts::NEWEST_START_TICK_TIME_DEVIATION_WEIGHT;
use shared::consts::SNAPSHOT_ARRIVAL_SIGMA_FACTOR;
use shared::util;
use shared::util::Mix;

use tick_time::TickInstant;
use super::ConnectionState;
use super::ServerInterface;
use super::TickInfo;
use self::InternalState::*;
use self::SnapshotState::*;

struct OnlineDistribution<T> where T:
        Copy
        + PartialOrd
        + Mix
        + Sub<T, Output=Duration>
        + Add<Duration, Output=T> {
    mean: T,
    variance: f64,
}

impl<T> OnlineDistribution<T> where T:
        Copy
        + PartialOrd
        + Mix
        + Sub<T, Output=Duration>
        + Add<Duration, Output=T> {
    fn new(sample: T) -> OnlineDistribution<T> {
        OnlineDistribution {
            mean: sample,
            variance: 0.0,
        }
    }

    fn add_sample(&mut self, sample: T) {
        let old_diff = if sample > self.mean {
            util::duration_as_float(sample - self.mean)
        } else {
            -util::duration_as_float(self.mean - sample)
        };
        /*if old_diff > Self::tick_tolerance_delay_float(self.variance) {
            println!(
                "WARNING: Snapshot {} arrived too late! | \
                    Deviation from mean: {:.2}ms | Tick tolerance delay: {:.2}ms",
                tick,
                old_diff * 1000.0,
                Self::tick_tolerance_delay_float(self.variance) * 1000.0
            );
        }*/
        self.mean = self.mean.mix(
            &sample,
            NEWEST_START_TICK_TIME_WEIGHT
        );
        let new_diff = if sample > self.mean {
            util::duration_as_float(sample - self.mean)
        } else {
            -util::duration_as_float(self.mean - sample)
        };
        self.variance = self.variance.mix(
            &(old_diff * new_diff),
            NEWEST_START_TICK_TIME_DEVIATION_WEIGHT
        );
    }

    fn mean(&self) -> T {
        self.mean
    }

    fn sigma_dev(&self, sigma_factor: f64) -> Duration {
        util::duration_from_float(self.variance.sqrt() * sigma_factor)
    }
}

struct Network {
    socket: UdpSocket,
}

impl Network {
    fn send(&self, msg: ClientMessage) {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        let amount = msg.pack(&mut buf).unwrap();
        self.socket.send(&buf[..amount]).unwrap();
    }

    fn recv(&self, read_time_out: Option<Duration>) -> Option<ServerMessage> {
        self.socket.set_read_timeout(read_time_out).unwrap();
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

enum SnapshotState {
    BeforeSnapshot,
    AfterSnapshot {
        start_tick_time_distribution: OnlineDistribution<Instant>,
        snapshots: HashMap<u64, Snapshot>,
        sent_inputs: HashMap<u64, CharacterInput>,
        oldest_snapshot_tick: u64,
        tick_info: TickInfo,
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
    internal_state: InternalState,
    network: Network
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
            let network = Network {
                socket,
            };
            network.send(ClientMessage::ConnectionRequest);
            let rsi = RemoteServerInterface {
                network,
                internal_state: Connecting,
            };
            Ok(rsi)
        })
    }

    fn on_snapshot(&mut self, snapshot: Snapshot) {
        let recv_time = Instant::now();
        let start_tick_time = recv_time - util::mult_duration(
            consts::tick_duration(),
            snapshot.tick(),
        );
        if let Connected { ref mut snapshot_state, .. } = self.internal_state {
            match snapshot_state {
                &mut BeforeSnapshot => *snapshot_state = AfterSnapshot {
                    start_tick_time_distribution: OnlineDistribution::new(start_tick_time),
                    tick_info: TickInfo { // not a real tick info :/ but we need one
                        tick: snapshot.tick(),
                        predicted_tick: snapshot.tick(),
                        tick_time: recv_time,
                        next_tick_time: recv_time, // must not lie in the future
                    },
                    oldest_snapshot_tick: snapshot.tick(),
                    snapshots: iter::once((snapshot.tick(), snapshot)).collect(),
                    sent_inputs: HashMap::new(),
                },
                &mut AfterSnapshot {
                    ref mut start_tick_time_distribution,
                    oldest_snapshot_tick,
                    ref mut snapshots,
                    ..
                } => {
                    start_tick_time_distribution.add_sample(start_tick_time);
                    if snapshot.tick() > oldest_snapshot_tick {
                        snapshots.insert(snapshot.tick(), snapshot);
                    } else {
                        println!("WARNING: Discarded snapshot {}!", snapshot.tick());
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
}

impl ServerInterface for RemoteServerInterface {
    fn do_tick(&mut self, model: &mut Model, character_input: CharacterInput) {
        let tick_lag = 20;// TODO use adaptive delay and prevent predicted tick decreasing
        if let Connected {
            my_player_id,
            snapshot_state: AfterSnapshot {
                ref start_tick_time_distribution,
                ref mut tick_info,
                ref mut oldest_snapshot_tick,
                ref mut snapshots,
                ref mut sent_inputs,
            },
        } = self.internal_state {
            // we add a multiple of the standard deviation of the snapshot arrival time distribution
            // to our ticks, as to make it likely that the snapshots will be on time
            let target_tick_instant = TickInstant::new(
                start_tick_time_distribution.mean()
                    + start_tick_time_distribution.sigma_dev(SNAPSHOT_ARRIVAL_SIGMA_FACTOR),
                tick_info.next_tick_time,
            );
            tick_info.tick += 1;
            tick_info.tick_time = tick_info.next_tick_time;
            let float_tick_diff = if target_tick_instant.tick > tick_info.tick {
                let tick_diff = target_tick_instant.tick - tick_info.tick;
                tick_diff as f64 + target_tick_instant.intra_tick
            } else {
                let tick_diff = tick_info.tick - target_tick_instant.tick;
                target_tick_instant.intra_tick - (tick_diff as f64)
            };

            let param1 = TICK_SPEED as f64 / 4.0;
            let param2 = TICK_SPEED as f64 / 4.0;
            let param3 = 0.2;
            let duration_factor;
            if float_tick_diff < 0.0 {
                duration_factor = (-float_tick_diff / param1 + 1.0).min(2.0);
            } else if float_tick_diff <= param2 {
                duration_factor = 1.0 - float_tick_diff / param2 * param3;
            } else {
                println!("WARNING: Jumping from {} to {}!",
                     tick_info.tick,
                     target_tick_instant.tick
                );
                tick_info.tick = target_tick_instant.tick;
                duration_factor = 1.0;
            }
            tick_info.next_tick_time = tick_info.tick_time + util::mult_duration_float(
                consts::tick_duration(),
                duration_factor,
            );
            tick_info.predicted_tick = tick_info.tick + tick_lag;

            // remove old snapshots and inputs
            {
                let mut new_oldest_snapshot_tick = *oldest_snapshot_tick;
                let mut t = tick_info.tick;
                while t >= *oldest_snapshot_tick {
                    if snapshots.contains_key(&t) {
                        new_oldest_snapshot_tick = t;
                        break;
                    }
                    t -= 1;
                }
                for t in *oldest_snapshot_tick..new_oldest_snapshot_tick {
                    let snapshot = snapshots.remove(&t);
                    if snapshot.is_none() {
                        println!("WARNING: Snapshot {} was never seen!", t);
                    }
                }
                for t in (*oldest_snapshot_tick + 1)..(new_oldest_snapshot_tick + 1) {
                    sent_inputs.remove(&t);
                }
                *oldest_snapshot_tick = new_oldest_snapshot_tick;
            }

            // send input
            let input_tick = tick_info.predicted_tick;
            let msg = ClientMessage::Input { tick: input_tick, input: character_input };
            self.network.send(msg);
            sent_inputs.insert(input_tick, character_input);

            // update model
            let oldest_snapshot = snapshots.get(oldest_snapshot_tick).unwrap();
            *model = oldest_snapshot.model().clone(); // TODO do this better
            let tick_diff = tick_info.tick - *oldest_snapshot_tick;
            if tick_diff > 0 {
                println!(
                    "WARNING: {} ticks ahead of snapshots! | \
                        Current tick: {} Tick of last snapshot: {} | \
                        Target tick: {}",
                    tick_diff, tick_info.tick, *oldest_snapshot_tick, target_tick_instant.tick
                );
            }
            for tick in (*oldest_snapshot_tick + 1)..(tick_info.tick + 1) {
                if let Some(input) = sent_inputs.get(&tick) {
                    model.set_character_input(my_player_id, *input);
                }
                model.do_tick();
            }
        }
    }

    fn handle_traffic(&mut self, until: Instant) {
        loop {
            let now = Instant::now();
            if until <= now {
                break;
            }
            if let Some(msg) = self.network.recv(Some(until - now)) {
                self.handle_message(msg);
            }
        }
    }

    fn connection_state(&self) -> ConnectionState {
        match self.internal_state {
            Connecting | Connected { snapshot_state: BeforeSnapshot, .. }
                => ConnectionState::Connecting,
            Disconnecting => ConnectionState::Disconnecting,
            Disconnected => ConnectionState::Disconnected,
            Connected { my_player_id, snapshot_state: AfterSnapshot { tick_info, .. } }
                => ConnectionState::Connected { my_player_id, tick_info }
        }
    }

    fn character_input(&self, tick: u64) -> Option<CharacterInput> {
        if let Connected {
            snapshot_state: AfterSnapshot { ref sent_inputs, .. },
            ..
        } = self.internal_state {
            sent_inputs.get(&tick).map(|input| *input)
        } else {
            None
        }
    }

    fn disconnect(&mut self) {
        self.network.send(ClientMessage::Leave);
        self.internal_state = Disconnecting;
    }
}