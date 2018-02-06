use std::time::Instant;

use shared::consts::TICK_SPEED;
use model::Model;
use model::world::character::CharacterInput;

pub trait ServerInterface {
    fn tick(&mut self, &mut Model, input: CharacterInput);
    fn get_tick(&self) -> u32;
    fn get_predicted_tick(&self) -> u32;
    fn get_next_tick_time(&self) -> Instant;
}

pub struct LocalServerInterface {
    start_tick_time: Instant,
    tick: u32,
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
        if self.is_first_tick {
            self.start_tick_time = now;
            self.tick = 0;
            self.is_first_tick = false;
        } else {
            let diff = now - self.start_tick_time;
            let sec_diff = diff.as_secs() as f64 + diff.subsec_nanos() as f64 * 1e-9;
            self.tick = (sec_diff * TICK_SPEED as f64).floor() as u32;
        }
        self.next_tick_time = self.start_tick_time
            + ::shared::consts::tick_length() * (self.tick + 1);
        model.set_character_input(input);
    }

    fn get_tick(&self) -> u32 {
        self.tick
    }

    fn get_predicted_tick(&self) -> u32 {
        self.tick
    }

    fn get_next_tick_time(&self) -> Instant {
        self.next_tick_time
    }
}