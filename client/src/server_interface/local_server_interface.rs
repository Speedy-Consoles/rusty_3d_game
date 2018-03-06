use std::time::Instant;

use shared::consts;
use shared::consts::TICK_SPEED;
use shared::util;
use shared::model::Model;
use shared::model::world::character::CharacterInput;

use super::ConnectionState;
use super::ConnectionState::*;
use super::ServerInterface;

pub struct LocalServerInterface {
    start_tick_time: Instant,
    tick: u64,
    next_tick_time: Instant,
    is_first_tick: bool,
    my_id: u64,
}

impl LocalServerInterface {
    pub fn new() -> LocalServerInterface {
        LocalServerInterface {
            start_tick_time: Instant::now(),
            tick: 0,
            next_tick_time: Instant::now(),
            is_first_tick: true,
            my_id: 0,
        }
    }
}

impl ServerInterface for LocalServerInterface {
    fn tick(&mut self, model: &mut Model, input: CharacterInput) {
        let now = Instant::now();
        let mut prev_tick = self.tick;
        if self.is_first_tick {
            self.start_tick_time = now;
            prev_tick = 0;
            self.tick = 0;
            self.my_id = model.spawn_character();
            self.is_first_tick = false;
        } else {
            let diff = now - self.start_tick_time;
            self.tick = util::elapsed_ticks(&diff, TICK_SPEED);
        }
        self.next_tick_time = self.start_tick_time
            + util::mult_duration(&consts::tick_interval(), self.tick + 1);

        let tick_diff = self.tick - prev_tick;
        for _ in 0..tick_diff {
            model.set_character_input(self.my_id, input);
            model.tick();
        }
    }

    fn get_tick(&self) -> u64 {
        self.tick
    }

    fn get_predicted_tick(&self) -> u64 {
        self.tick
    }

    fn get_intra_tick(&self) -> f64 {
        let now = Instant::now();
        let diff = now - self.start_tick_time
            - util::mult_duration(&consts::tick_interval(), self.tick);
        let sec_diff = diff.as_secs() as f64 + diff.subsec_nanos() as f64 * 1e-9;
        sec_diff * TICK_SPEED as f64
    }

    fn get_next_tick_time(&self) -> Instant {
        self.next_tick_time
    }

    fn get_my_id(&self) -> Option<u64> {
        if self.is_first_tick {
            None
        } else {
            Some(self.my_id)
        }
    }

    fn get_character_input(&self, _tick: u64) -> Option<CharacterInput> {
        None
    }

    fn get_connection_state(&self) -> ConnectionState {
        Connected
    }
}