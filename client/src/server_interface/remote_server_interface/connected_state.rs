use std::time::Instant;
use std::collections::HashMap;
use std::iter;

use shared::tick_time::TickInstant;
use shared::tick_time::TickRate;
use shared::model::Model;
use shared::model::world::World;
use shared::model::world::character::CharacterInput;
use shared::net::socket::ConId;
use shared::net::ReliableServerMessage;
use shared::net::ReliableServerMessage::*;
use shared::net::UnreliableServerMessage;
use shared::net::UnreliableServerMessage::*;
use shared::net::UnreliableClientMessage::*;
use shared::net::Snapshot;
use shared::consts;
use shared::consts::TICK_SPEED;
use shared::consts::NEWEST_START_TICK_TIME_WEIGHT;
use shared::consts::SNAPSHOT_ARRIVAL_SIGMA_FACTOR;
use shared::consts::NEWEST_START_PREDICTED_TICK_TIME_WEIGHT;
use shared::consts::INPUT_ARRIVAL_SIGMA_FACTOR;
use shared::util;
use shared::online_distribution::OnlineDistribution;

use server_interface::remote_server_interface::ClientSocket;
use server_interface::ConnectionState;

struct AfterSnapshotData {
    tick: u64,
    predicted_tick: u64,
    predicted_tick_decrease: f64,
    tick_time: Instant,
    next_tick_time: Instant,
    model: Model,
    predicted_world: World,
    start_tick_time_distribution: OnlineDistribution<Instant>,
    oldest_snapshot_tick: u64,
    snapshots: HashMap<u64, Snapshot>,
    start_predicted_tick_distribution: OnlineDistribution<Instant>,
    sent_inputs: HashMap<u64, CharacterInput>,
    sent_input_times: HashMap<u64, Instant>,
}

impl AfterSnapshotData {
    fn new(snapshot: Snapshot, recv_time: Instant) -> AfterSnapshotData {
        let start_tick_time = recv_time - snapshot.tick() / TICK_SPEED;
        let start_predicted_tick_time = start_tick_time - consts::initial_lag_assumption();
        AfterSnapshotData {
            tick: snapshot.tick(),
            predicted_tick: snapshot.tick(),
            predicted_tick_decrease: 0.0,
            tick_time: recv_time - 1 / TICK_SPEED,
            next_tick_time: recv_time,
            model: Model::new(), // maybe don't initialize this yet
            predicted_world: World::new(), // maybe don't initialize this yet
            start_tick_time_distribution: OnlineDistribution::new(start_tick_time),
            oldest_snapshot_tick: snapshot.tick(),
            snapshots: iter::once((snapshot.tick(), snapshot)).collect(),
            // TODO start_predicted_tick_time should be determined by ping from connection request instead
            start_predicted_tick_distribution: OnlineDistribution::new(start_predicted_tick_time),
            sent_inputs: HashMap::new(),
            sent_input_times: HashMap::new(),
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
                    "DEBUG: Snapshot {} arrived too late! | \
                        Deviation from mean: {:.2}ms | Tick tolerance delay: {:.2}ms",
                    snapshot.tick(),
                    util::duration_as_float(diff) * 1000.0,
                    util::duration_as_float(sigma_dev) * 1000.0,
                );
            }
        }
        self.start_tick_time_distribution.add_sample(
            start_tick_time,
            NEWEST_START_TICK_TIME_WEIGHT
        );
        // TODO ignore snapshots with insanely high tick
        if snapshot.tick() > self.oldest_snapshot_tick {
            self.snapshots.insert(snapshot.tick(), snapshot);
        } else {
            println!("DEBUG: Discarded snapshot {}!", snapshot.tick());
        }
    }

    fn on_input_ack(&mut self, input_tick: u64, arrival_tick_instant: TickInstant) {
        if let Some(send_time) = self.sent_input_times.get(&input_tick) {
            let start_predicted_tick_time = *send_time
                - (arrival_tick_instant - TickInstant::zero()) / TICK_SPEED;
            self.start_predicted_tick_distribution.add_sample(
                start_predicted_tick_time,
                NEWEST_START_PREDICTED_TICK_TIME_WEIGHT,
            );
        } else {
            println!("DEBUG: Received input ack of unknown input!");
        }
    }

    fn update_tick(&mut self) {
        self.tick += 1;
        self.tick_time = self.next_tick_time;
        // we add a multiple of the standard deviation of the snapshot arrival time distribution
        // to our ticks, to make it likely that the snapshots will be on time
        let target_tick_instant = TickInstant::from_start_tick(
            self.start_tick_time_distribution.mean()
                + self.start_tick_time_distribution.sigma_dev(SNAPSHOT_ARRIVAL_SIGMA_FACTOR),
            self.tick_time,
            TICK_SPEED,
        );
        let float_tick_diff = if target_tick_instant.tick > self.tick {
            let tick_diff = target_tick_instant.tick - self.tick;
            tick_diff as f64 + target_tick_instant.intra_tick
        } else {
            let tick_diff = self.tick - target_tick_instant.tick;
            target_tick_instant.intra_tick - (tick_diff as f64)
        };

        let min_factor = 0.5;
        let max_factor = 2.0;
        let factor_factor = 0.05;
        let jump_threshold = 30.0;
        let mut speed_factor;
        if float_tick_diff < jump_threshold {
            // TODO replace this simple linear function with something more thoughtful
            speed_factor = 1.0 + float_tick_diff * factor_factor;
        } else {
            println!(
                "DEBUG: Jumping from {} to {}!",
                 self.tick,
                 target_tick_instant.tick,
            );
            self.tick = target_tick_instant.tick;
            speed_factor = 1.0;
        };
        speed_factor = speed_factor.min(max_factor).max(min_factor);
        let tick_rate = TickRate::from_per_second(
            (TICK_SPEED.per_second() as f64 * speed_factor) as u64
        );
        self.next_tick_time = self.tick_time + 1 / tick_rate;
    }

    fn send_and_save_input(&mut self, character_input: CharacterInput, socket: &mut ClientSocket,
                           con_id: ConId) {
        self.predicted_tick += 1;
        let send_time = Instant::now();
        // we add a multiple of the standard deviation of the input arrival time distribution
        // to our input ticks, to make it likely that it will be on time
        let arrival_tick_instant = TickInstant::from_start_tick(
            self.start_predicted_tick_distribution.mean()
                - self.start_predicted_tick_distribution.sigma_dev(INPUT_ARRIVAL_SIGMA_FACTOR),
            send_time,
            TICK_SPEED,
        );
        let target_predicted_tick = arrival_tick_instant.tick + 1;

        let factor = 0.001;
        if target_predicted_tick >= self.predicted_tick {
            if target_predicted_tick > self.predicted_tick {
                println!(
                    "DEBUG: predicted tick jumping by {}!",
                    target_predicted_tick - self.predicted_tick
                );
            }
            self.predicted_tick_decrease = 0.0;
            self.predicted_tick = target_predicted_tick;
        } else {
            self.predicted_tick_decrease += (
                (self.predicted_tick - target_predicted_tick) as f64 * factor
            ).min(1.0);
            if self.predicted_tick_decrease >= 1.0 {
                self.predicted_tick -= 1;
                self.predicted_tick_decrease -= 1.0;
            }
        }

        let msg = InputMessage {
            tick: self.predicted_tick,
            input: character_input,
        };
        // TODO if we resend any input, the server will send another ack and we might calculate a wrong input delay
        socket.send_to_unreliable(con_id, msg);
        self.sent_input_times.insert(self.predicted_tick, send_time);
        self.sent_inputs.insert(self.predicted_tick, character_input);
    }

    fn remove_old_snapshots_and_inputs(&mut self) {
        // TODO make this function more efficient
        let mut new_oldest_snapshot_tick = self.oldest_snapshot_tick;
        let mut t = self.tick;
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
                println!("DEBUG: Snapshot {} was never seen!", t);
            }
        }
        for t in (self.oldest_snapshot_tick + 1)..(new_oldest_snapshot_tick + 1) {
            self.sent_inputs.remove(&t);
        }
        self.oldest_snapshot_tick = new_oldest_snapshot_tick;
        let now = Instant::now();
        self.sent_input_times.retain(|_, time| now - *time < consts::max_input_keep_time() )
    }

    fn update_model(&mut self, my_player_id: u64) {
        let oldest_snapshot = self.snapshots.get(&self.oldest_snapshot_tick).unwrap();
        self.model = oldest_snapshot.model().clone(); // TODO do this better
        let tick_diff = self.tick - self.oldest_snapshot_tick;
        if tick_diff > 0 {
            println!(
                "DEBUG: {} ticks ahead of snapshots! | \
                        Current tick: {} | Tick of oldest snapshot: {}",
                tick_diff,
                self.tick,
                self.oldest_snapshot_tick
            );
        }
        for tick in (self.oldest_snapshot_tick + 1)..(self.tick + 1) {
            if let Some(input) = self.sent_inputs.get(&tick) {
                self.model.set_character_input(my_player_id, *input);
            }
            self.model.do_tick();
        }

        self.predicted_world = self.model.world().clone();
        for tick in (self.tick + 1)..(self.predicted_tick + 1) {
            if let Some(&input) = self.sent_inputs.get(&tick) {
                self.predicted_world.set_character_input(my_player_id, input);
            }
            self.predicted_world.do_tick();
        }
    }
}

pub struct ConnectedState {
    my_player_id: u64,
    after_snapshot_data: Option<AfterSnapshotData>,
}

impl ConnectedState {
    pub fn new(my_player_id: u64) -> ConnectedState {
        ConnectedState {
            my_player_id,
            after_snapshot_data: None,
        }
    }

    pub fn do_tick(&mut self, character_input: CharacterInput, socket: &mut ClientSocket,
                   con_id: ConId) {
        // TODO check if server is not sending snapshots
        // TODO check if server is not acking input
        if let Some(ref mut data) = self.after_snapshot_data {
            data.update_tick();
            data.send_and_save_input(character_input, socket, con_id);
            data.remove_old_snapshots_and_inputs();
            data.update_model(self.my_player_id);
        }
    }

    pub fn next_tick_time(&self) -> Option<Instant> {
        match self.after_snapshot_data {
            None => None,
            Some(ref data) => Some(data.next_tick_time),
        }
    }

    pub fn connection_state(&self) -> ConnectionState {
        match self.after_snapshot_data {
            None => ConnectionState::Connecting,
            Some(ref data) => ConnectionState::Connected {
                my_player_id: self.my_player_id,
                tick_instant: TickInstant::from_interval(
                    data.tick, data.tick_time,
                    data.next_tick_time,
                    Instant::now()
                ),
                model: &data.model,
                predicted_world: &data.predicted_world,
            }
        }
    }

    pub fn handle_unreliable_message(&mut self, msg: UnreliableServerMessage) {
        let recv_time = Instant::now();
        match msg {
            SnapshotMessage(snapshot) => match self.after_snapshot_data {
                None => self.after_snapshot_data = Some(
                    AfterSnapshotData::new(snapshot, recv_time)
                ),
                Some(ref mut data) => data.on_snapshot(snapshot, recv_time),
            },
            InputAck { input_tick, arrival_tick_instant } => {
                if let Some(ref mut data) = self.after_snapshot_data {
                    data.on_input_ack(input_tick, arrival_tick_instant);
                }
            }
        }
    }

    pub fn handle_reliable_message(&mut self, msg: ReliableServerMessage) {
        match msg {
            ConnectionClose => (), // should be handled before
        }
    }
}