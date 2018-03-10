use std::time::Instant;
use std::thread;

use shared::consts;
use shared::util;
use shared::model::Model;
use shared::model::world::character::CharacterInput;

use super::ConnectionState;
use super::ServerInterface;
use super::TickInfo;
use self::InternalState::*;

enum InternalState {
    BeforeFirstTick,
    AfterFirstTick {
        start_tick_time: Instant,
        my_player_id: u64,
        tick_info: TickInfo,
    },
    AfterDisconnect,
}
pub struct LocalServerInterface {
    internal_state: InternalState,
}

impl LocalServerInterface {
    pub fn new() -> LocalServerInterface {
        LocalServerInterface {
            internal_state: BeforeFirstTick,
        }
    }
}

impl ServerInterface for LocalServerInterface {
    fn tick(&mut self, model: &mut Model, input: CharacterInput) {
        let tick_diff;
        let player_id;
        match self.internal_state {
            BeforeFirstTick => {
                player_id = model.add_player(String::from("Player"));
                tick_diff = 1;
                let now = Instant::now();
                self.internal_state = AfterFirstTick {
                    start_tick_time: now,
                    my_player_id: player_id,
                    tick_info: TickInfo {
                        tick: 0,
                        tick_time: now,
                        predicted_tick: 0,
                        next_tick_time: now + consts::tick_duration(),
                    }
                };
            },
            AfterFirstTick { start_tick_time, my_player_id, ref mut tick_info } => {
                player_id = my_player_id;
                let prev_tick = tick_info.tick;
                tick_info.tick += 1; // TODO allow tick skipping
                tick_info.predicted_tick = tick_info.tick;
                tick_info.tick_time = tick_info.next_tick_time;
                tick_info.next_tick_time = start_tick_time
                    + util::mult_duration(consts::tick_duration(), tick_info.tick + 1);
                tick_diff = tick_info.tick - prev_tick;
            },
            AfterDisconnect => return,
        }

        for _ in 0..tick_diff {
            model.set_character_input(player_id, input);
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

    fn get_connection_state(&self) -> ConnectionState {
        match self.internal_state {
            BeforeFirstTick => ConnectionState::Connecting,
            AfterFirstTick { my_player_id, tick_info, .. }
            => ConnectionState::Connected { my_player_id, tick_info },
            AfterDisconnect => ConnectionState::Disconnected,
        }
    }

    fn get_character_input(&self, _tick: u64) -> Option<CharacterInput> {
        None
    }

    fn disconnect(&mut self) {
        self.internal_state = AfterDisconnect
    }
}