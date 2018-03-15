mod local_server_interface;
mod remote_server_interface;

use std::time::Instant;

use shared::tick_time::TickInstant;
use shared::model::Model;
use shared::model::world::World;
use shared::model::world::character::CharacterInput;

pub use self::local_server_interface::*;
pub use self::remote_server_interface::*;

#[derive(Clone, Copy)]
pub enum DisconnectedReason<'a> {
    NetworkError,
    UserDisconnect,
    Kicked {
        kick_message: &'a str,
    },
    TimedOut,
}

#[derive(Clone, Copy)]
pub enum ConnectionState<'a> {
    Connecting,
    Connected {
        my_player_id: u64,
        tick_instant: TickInstant,
        model: &'a Model,
        predicted_world: &'a World,
    },
    Disconnecting,
    Disconnected(DisconnectedReason<'a>),
}

pub trait ServerInterface {
    fn do_tick(&mut self, input: CharacterInput);
    fn handle_traffic(&mut self, until: Instant);
    fn connection_state(&self) -> ConnectionState;
    fn next_tick_time(&self) -> Option<Instant>;
    fn disconnect(&mut self);
}