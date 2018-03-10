mod local_server_interface;
mod remote_server_interface;

use std::time::Instant;

use shared::model::Model;
use shared::model::world::character::CharacterInput;

pub use self::local_server_interface::*;
pub use self::remote_server_interface::*;

#[derive(Clone, Copy)]
pub enum ConnectionState {
    Connecting,
    Connected {
        my_player_id: u64,
        tick_info: TickInfo,
    },
    Disconnecting,
    Disconnected,
}

#[derive(Debug, Clone, Copy)]
pub struct TickInfo {
    pub tick: u64,
    pub predicted_tick: u64,
    pub tick_time: Instant,
    pub next_tick_time: Instant,
}

pub trait ServerInterface {
    fn tick(&mut self, model: &mut Model, input: CharacterInput);
    fn handle_traffic(&mut self, until: Instant);
    fn get_connection_state(&self) -> ConnectionState;
    fn get_character_input(&self, tick: u64) -> Option<CharacterInput>;
    fn disconnect(&mut self);
}