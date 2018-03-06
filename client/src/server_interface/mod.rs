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
    Connected,
    Disconnecting,
    Disconnected,
}

pub trait ServerInterface {
    fn tick(&mut self, model: &mut Model, input: CharacterInput);
    fn get_tick(&self) -> u64;
    fn handle_traffic(&mut self, until: Instant);
    fn get_predicted_tick(&self) -> u64;
    fn get_intra_tick(&self) -> f64;
    fn get_next_tick_time(&self) -> Instant;
    fn get_my_id(&self) -> Option<u64>;
    fn get_character_input(&self, tick: u64) -> Option<CharacterInput>;
    fn get_connection_state(&self) -> ConnectionState;
}