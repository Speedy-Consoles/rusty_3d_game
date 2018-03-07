use std::time::Instant;
use std::thread;

use shared::consts;
use shared::consts::TICK_SPEED;
use shared::util;
use shared::model::Model;
use shared::model::world::character::CharacterInput;

use super::ConnectionState;
use super::ConnectionState::*;
use super::ServerInterface;
use super::TickInfo;

pub struct LocalServerInterface {
    start_tick_time: Instant,
    tick_info: Option<TickInfo>,
    my_player_id: Option<u64>,
}

impl LocalServerInterface {
    pub fn new() -> LocalServerInterface {
        LocalServerInterface {
            start_tick_time: Instant::now(),
            tick_info: None,
            my_player_id: None,
        }
    }
}

impl ServerInterface for LocalServerInterface {
    fn tick(&mut self, model: &mut Model, input: CharacterInput) {
        let now = Instant::now();
        let tick_diff;
        if let Some(ref mut tick_info) = self.tick_info {
            let prev_tick = tick_info.tick;
            let diff = now - self.start_tick_time;
            tick_info.tick = util::elapsed_ticks(diff, TICK_SPEED);
            tick_info.tick_time = self.start_tick_time
                + util::mult_duration(consts::tick_interval(), tick_info.tick);
            tick_diff = tick_info.tick - prev_tick;
        } else {
            self.start_tick_time = now;
            self.tick_info = Some(TickInfo {
                tick: 0,
                tick_time: now,
            });
            tick_diff = 1;
            self.my_player_id = Some(model.add_player(String::from("Player")));
        }

        let my_player_id = self.my_player_id.unwrap();
        for _ in 0..tick_diff {
            model.set_character_input(my_player_id, input);
            model.tick();
        }
    }

    fn handle_traffic(&mut self, until: Instant) {
        loop {
            let now = Instant::now();
            if until <= now {
                break;
            }
            thread::sleep(until - now);
        }
    }

    fn get_tick_info(&self) -> Option<TickInfo> {
        self.tick_info
    }

    fn get_tick_lag(&self) -> u64 {
        0
    }

    fn get_my_player_id(&self) -> Option<u64> {
        self.my_player_id
    }

    fn get_character_input(&self, _tick: u64) -> Option<CharacterInput> {
        None
    }

    fn get_connection_state(&self) -> ConnectionState {
        Connected
    }
}