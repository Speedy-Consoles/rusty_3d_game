mod local_server_interface;
mod remote_server_interface;

use std::time::Instant;

use shared::tick_time::TickInstant;
use shared::util;
use shared::model::Model;
use shared::model::world::World;
use shared::model::world::character::CharacterInput;

pub use self::local_server_interface::*;
pub use self::remote_server_interface::*;

#[derive(Clone, Copy)]
pub enum ConnectionState<'a> {
    Connecting,
    Connected {
        my_player_id: u64,
        tick_info: TickInfo,
        model: &'a Model,
        predicted_world: &'a World,
    },
    Disconnecting,
    Disconnected,
}

#[derive(Debug, Clone, Copy)]
pub struct TickInfo {
    pub tick: u64,
    pub tick_time: Instant,
    pub next_tick_time: Instant,
}

impl TickInfo {
    pub fn now(&self) -> TickInstant {
        let part_dur = Instant::now() - self.tick_time;
        let whole_dur = self.next_tick_time - self.tick_time;
        TickInstant {
            tick: self.tick,
            intra_tick: util::duration_as_float(part_dur) / util::duration_as_float(whole_dur),
        }
    }
}

pub trait ServerInterface {
    fn do_tick(&mut self, input: CharacterInput);
    fn handle_traffic(&mut self, until: Instant);
    fn connection_state(&self) -> ConnectionState;
    fn disconnect(&mut self);
}