use std::time::Instant;
use std::time::Duration;
use std::collections::HashMap;
use std::iter;
use std::ops::Sub;
use std::ops::Add;

use shared::tick_time::TickInstant;
use shared::tick_time::TickRate;
use shared::model::Model;
use shared::model::world::World;
use shared::model::world::character::CharacterInput;
use shared::net::ConnectedClientMessage;
use shared::net::Snapshot;
use shared::consts::TICK_SPEED;
use shared::consts::NEWEST_START_TICK_TIME_WEIGHT;
use shared::consts::NEWEST_START_TICK_TIME_DEVIATION_WEIGHT;
use shared::consts::SNAPSHOT_ARRIVAL_SIGMA_FACTOR;
use shared::util;
use shared::util::Mix;

use server_interface::remote_server_interface::socket::Socket;
use server_interface::ConnectionState;
use server_interface::TickInfo;

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

pub struct AfterSnapshotState {
    my_player_id: u64,
    start_tick_time_distribution: OnlineDistribution<Instant>,
    snapshots: HashMap<u64, Snapshot>,
    sent_inputs: HashMap<u64, CharacterInput>,
    oldest_snapshot_tick: u64,
    tick_info: TickInfo,
    predicted_tick: u64,
    model: Model,
    predicted_world: World,
}

impl AfterSnapshotState {
    pub fn new(my_player_id: u64, snapshot: Snapshot, recv_time: Instant) -> AfterSnapshotState {
        let start_tick_time = recv_time - snapshot.tick() / TICK_SPEED;
        AfterSnapshotState {
            my_player_id,
            start_tick_time_distribution: OnlineDistribution::new(start_tick_time),
            tick_info: TickInfo { // not a real tick info :/ but we need one
                tick: snapshot.tick(),
                tick_time: recv_time,
                next_tick_time: recv_time, // must not lie in the future
            },
            predicted_tick: snapshot.tick(),
            model: Model::new(), // maybe don't initialize this yet
            predicted_world: World::new(), // maybe don't initialize this yet
            oldest_snapshot_tick: snapshot.tick(),
            snapshots: iter::once((snapshot.tick(), snapshot)).collect(),
            sent_inputs: HashMap::new(),
        }
    }

    pub fn connection_state(&self) -> ConnectionState {
        ConnectionState::Connected {
            my_player_id: self.my_player_id,
            tick_info: self.tick_info,
            model: &self.model,
            predicted_world: &self.predicted_world,
        }
    }

    pub fn on_snapshot(&mut self, snapshot: Snapshot, recv_time: Instant) {
        let start_tick_time = recv_time - snapshot.tick() / TICK_SPEED;
        if false {
            let sigma_dev = self.start_tick_time_distribution.sigma_dev(SNAPSHOT_ARRIVAL_SIGMA_FACTOR);
            let limit = self.start_tick_time_distribution.mean() + sigma_dev;
            if start_tick_time > limit {
                let diff = start_tick_time - self.start_tick_time_distribution.mean();
                println!(
                    "WARNING: Snapshot {} arrived too late! | \
                        Deviation from mean: {:.2}ms | Tick tolerance delay: {:.2}ms",
                    snapshot.tick(),
                    util::duration_as_float(diff) * 1000.0,
                    util::duration_as_float(sigma_dev) * 1000.0,
                );
            }
        }
        self.start_tick_time_distribution.add_sample(start_tick_time);
        if snapshot.tick() > self.oldest_snapshot_tick {
            self.snapshots.insert(snapshot.tick(), snapshot);
        } else {
            println!("WARNING: Discarded snapshot {}!", snapshot.tick());
        }
    }

    pub fn do_tick(&mut self, network: &Socket, character_input: CharacterInput) {
        self.update_tick_info();
        self.update_predict_tick();
        self.remove_old_snapshots_and_inputs();
        self.send_and_save_input(network, character_input);
        self.update_model();
    }

    fn update_tick_info(&mut self) {
        // we add a multiple of the standard deviation of the snapshot arrival time distribution
        // to our ticks, to make it likely that the snapshots will be on time
        let target_tick_instant = TickInstant::new(
            self.start_tick_time_distribution.mean()
                + self.start_tick_time_distribution.sigma_dev(SNAPSHOT_ARRIVAL_SIGMA_FACTOR),
            self.tick_info.next_tick_time,
            TICK_SPEED,
        );
        self.tick_info.tick += 1;
        self.tick_info.tick_time = self.tick_info.next_tick_time;
        let float_tick_diff = if target_tick_instant.tick > self.tick_info.tick {
            let tick_diff = target_tick_instant.tick - self.tick_info.tick;
            tick_diff as f64 + target_tick_instant.intra_tick
        } else {
            let tick_diff = self.tick_info.tick - target_tick_instant.tick;
            target_tick_instant.intra_tick - (tick_diff as f64)
        };

        let min_factor = 0.5;
        let max_factor = 2.0;
        let factor_factor = 0.5;
        let jump_threshold = 30.0;
        let mut speed_factor;
        if float_tick_diff < jump_threshold {
            // TODO replace this simple linear function with something more thoughtful
            speed_factor = 1.0 + float_tick_diff * factor_factor
        } else {
            println!("WARNING: Jumping from {} to {}!",
                     self.tick_info.tick,
                     target_tick_instant.tick
            );
            self.tick_info.tick = target_tick_instant.tick;
            speed_factor = 1.0;
        };
        speed_factor = speed_factor.min(max_factor).max(min_factor);
        let tick_rate = TickRate::from_per_second(
            (TICK_SPEED.per_second() as f64 * speed_factor) as u64
        );
        self.tick_info.next_tick_time = self.tick_info.tick_time + 1 / tick_rate;
    }

    fn update_predict_tick(&mut self) {
        self.predicted_tick = self.tick_info.tick + 20; // TODO use adaptive delay and prevent predicted tick decreasing
    }

    fn remove_old_snapshots_and_inputs(&mut self) {
        let mut new_oldest_snapshot_tick = self.oldest_snapshot_tick;
        let mut t = self.tick_info.tick;
        while t > self.oldest_snapshot_tick {
            if self.snapshots.contains_key(&t) {
                new_oldest_snapshot_tick = t;
                break;
            }
            t -= 1;
        }
        for t in self.oldest_snapshot_tick..new_oldest_snapshot_tick {
            let snapshot = self.snapshots.remove(&t);
            if snapshot.is_none() {
                println!("WARNING: Snapshot {} was never seen!", t);
            }
        }
        for t in (self.oldest_snapshot_tick + 1)..(new_oldest_snapshot_tick + 1) {
            self.sent_inputs.remove(&t);
        }
        self.oldest_snapshot_tick = new_oldest_snapshot_tick;
    }

    fn send_and_save_input(&mut self, socket: &Socket, character_input: CharacterInput) {
        let msg = ConnectedClientMessage::Input {
            tick: self.predicted_tick,
            input: character_input
        };
        socket.send_connected(msg);
        self.sent_inputs.insert(self.predicted_tick, character_input);
    }

    fn update_model(&mut self) {
        let oldest_snapshot = self.snapshots.get(&self.oldest_snapshot_tick).unwrap();
        self.model = oldest_snapshot.model().clone(); // TODO do this better
        let tick_diff = self.tick_info.tick - self.oldest_snapshot_tick;
        if tick_diff > 0 {
            println!(
                "WARNING: {} ticks ahead of snapshots! | \
                        Current tick: {} | Tick of oldest snapshot: {}",
                tick_diff,
                self.tick_info.tick,
                self.oldest_snapshot_tick
            );
        }
        for tick in (self.oldest_snapshot_tick + 1)..(self.tick_info.tick + 1) {
            if let Some(input) = self.sent_inputs.get(&tick) {
                self.model.set_character_input(self.my_player_id, *input);
            }
            self.model.do_tick();
        }

        self.predicted_world = self.model.world().clone();
        for tick in (self.tick_info.tick + 1)..(self.predicted_tick + 1) {
            if let Some(&input) = self.sent_inputs.get(&tick) {
                self.predicted_world.set_character_input(self.my_player_id, input);
            }
            self.predicted_world.do_tick();
        }
    }
}