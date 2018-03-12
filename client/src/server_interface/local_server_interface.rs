use std::time::Instant;
use std::thread;

use shared::consts::TICK_SPEED;
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
        model: Model,
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
    fn do_tick(&mut self, input: CharacterInput) {
        let tick_diff;
        match self.internal_state {
            BeforeFirstTick => {
                let now = Instant::now();
                let mut model = Model::new();
                let my_player_id = model.add_player(String::from("Player"));
                model.set_character_input(my_player_id, input);
                model.do_tick();

                self.internal_state = AfterFirstTick {
                    start_tick_time: now,
                    my_player_id,
                    tick_info: TickInfo {
                        tick: 0,
                        tick_time: now,
                        next_tick_time: now + 1 / TICK_SPEED,
                    },
                    model,
                };
            },
            AfterFirstTick { start_tick_time, my_player_id, ref mut tick_info, ref mut model } => {
                let prev_tick = tick_info.tick;
                tick_info.tick += 1; // TODO allow tick skipping
                tick_info.tick_time = tick_info.next_tick_time;
                tick_info.next_tick_time = start_tick_time + (tick_info.tick + 1) / TICK_SPEED;
                tick_diff = tick_info.tick - prev_tick;

                model.set_character_input(my_player_id, input);
                for _ in 0..tick_diff {
                    model.do_tick();
                }
            },
            AfterDisconnect => (),
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

    fn connection_state(&self) -> ConnectionState {
        match self.internal_state {
            BeforeFirstTick => ConnectionState::Connecting,
            AfterFirstTick { my_player_id, tick_info, ref model, .. }
            => ConnectionState::Connected {
                my_player_id,
                tick_info,
                model,
                predicted_world: model.world(),
            },
            AfterDisconnect => ConnectionState::Disconnected,
        }
    }

    fn disconnect(&mut self) {
        self.internal_state = AfterDisconnect
    }
}