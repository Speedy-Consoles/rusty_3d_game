use std::time::Instant;
use std::thread;

use shared::consts::TICK_SPEED;
use shared::tick_time::TickInstant;
use shared::model::Model;
use shared::model::world::character::CharacterInput;

use super::ConnectionState;
use super::DisconnectedReason;
use super::ServerInterface;
use self::InternalState::*;

enum InternalState {
    BeforeFirstTick,
    AfterFirstTick {
        start_tick_time: Instant,
        my_player_id: u64,
        tick: u64,
        tick_time: Instant,
        next_tick_time: Instant,
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
        let now = Instant::now();
        match self.internal_state {
            BeforeFirstTick => {
                let mut model = Model::new();
                let my_player_id = model.add_player(String::from("Player"));
                model.set_character_input(my_player_id, input);
                model.do_tick();

                self.internal_state = AfterFirstTick {
                    start_tick_time: now,
                    my_player_id,
                    tick: 0,
                    tick_time: now,
                    next_tick_time: now + 1 / TICK_SPEED,
                    model,
                };
            },
            AfterFirstTick {
                start_tick_time,
                my_player_id,
                ref mut tick,
                ref mut tick_time,
                ref mut next_tick_time,
                ref mut model,
            } => {
                let prev_tick = *tick;
                let tick_instant = TickInstant::from_start_tick(start_tick_time, now, TICK_SPEED);
                *tick = tick_instant.tick;
                *tick_time = now;
                *next_tick_time = start_tick_time + (*tick + 1) / TICK_SPEED;
                // if we missed too many ticks, don't try to catch up...
                let tick_diff = (*tick - prev_tick).min(TICK_SPEED.per_second() / 2);

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
            AfterFirstTick {
                my_player_id,
                tick,
                ref model,
                tick_time,
                next_tick_time,
                ..
            } => {
                ConnectionState::Connected {
                    my_player_id,
                    tick_instant: TickInstant::from_interval(
                        tick,
                        tick_time,
                        next_tick_time,
                        Instant::now()
                    ),
                    model,
                    predicted_world: model.world(),
                }
            },
            AfterDisconnect => ConnectionState::Disconnected(DisconnectedReason::UserDisconnect),
        }
    }

    fn next_tick_time(&self) -> Option<Instant> {
        match self.internal_state {
            BeforeFirstTick => Some(Instant::now()),
            AfterFirstTick { ref start_tick_time, ref tick, .. } => {
                Some(*start_tick_time + (tick + 1) / TICK_SPEED)
            },
            AfterDisconnect => None,
        }
    }

    fn disconnect(&mut self) {
        self.internal_state = AfterDisconnect
    }
}