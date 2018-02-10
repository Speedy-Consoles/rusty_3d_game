use std::time::Instant;

use shared::consts;
use shared::consts::TICK_SPEED;
use shared::util;
use shared::model::Model;
use shared::model::world::character::CharacterInput;

pub trait ServerInterface {
    fn tick(&mut self, &mut Model, input: CharacterInput);
    fn get_tick(&self) -> u64;
    fn get_predicted_tick(&self) -> u64;
    fn get_intra_tick(&self) -> f64;
    fn get_next_tick_time(&self) -> Instant;
}

pub struct LocalServerInterface {
    start_tick_time: Instant,
    tick: u64,
    next_tick_time: Instant,
    is_first_tick: bool,
}

impl LocalServerInterface {
    pub fn new() -> LocalServerInterface {
        LocalServerInterface {
            start_tick_time: Instant::now(),
            tick: 0,
            next_tick_time: Instant::now(),
            is_first_tick: true,
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
            self.is_first_tick = false;
        } else {
            let diff = now - self.start_tick_time;
            let sec_diff = diff.as_secs() as f64 + diff.subsec_nanos() as f64 * 1e-9;
            self.tick = (sec_diff * TICK_SPEED as f64).floor() as u64;
        }
        self.next_tick_time = self.start_tick_time
            + util::mult_duration(&consts::tick_interval(), self.tick + 1);

        let tick_diff = self.tick - prev_tick;
        for _ in 0..tick_diff {
            model.set_character_input(input);
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
}