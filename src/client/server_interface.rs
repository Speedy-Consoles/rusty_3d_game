use std::time::Instant;

use consts::TICK_SPEED;
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
}

impl LocalServerInterface {
    pub fn new() -> LocalServerInterface {
        LocalServerInterface {
            start_tick_time: Instant::now(),
            tick: 0,
            next_tick_time: Instant::now(),
        }
    }
}

impl ServerInterface for LocalServerInterface {
    fn tick(&mut self, model: &mut Model, input: CharacterInput) {
        let now = Instant::now();
        let diff = now - self.start_tick_time;
        let sec_diff = diff.as_secs() as f64 + diff.subsec_nanos() as f64 * 10e-9 ;
        self.tick = (sec_diff / TICK_SPEED as f64).floor() as u32;
        self.next_tick_time = self.start_tick_time + ::consts::tick_length() * self.tick;
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