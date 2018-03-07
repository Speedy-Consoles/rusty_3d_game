mod local_server_interface;
mod remote_server_interface;

use std::time::Instant;

use shared::consts::TICK_SPEED;
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

#[derive(Clone, Copy)]
pub struct TickInfo {
    pub tick: u64,
    pub tick_time: Instant,
}

impl TickInfo {
    pub fn get_intra_tick(&self) -> f64 {
        let diff = Instant::now() - self.tick_time;
        let sec_diff = diff.as_secs() as f64 + diff.subsec_nanos() as f64 * 1e-9;
        sec_diff * TICK_SPEED as f64
    }
}

pub trait ServerInterface {
    fn tick(&mut self, model: &mut Model, input: CharacterInput);
    fn handle_traffic(&mut self, until: Instant);
    fn get_tick_info(&self) -> Option<TickInfo>;
    fn get_tick_lag(&self) -> Option<u64>;
    fn get_my_player_id(&self) -> Option<u64>;
    fn get_character_input(&self, tick: u64) -> Option<CharacterInput>;
    fn get_connection_state(&self) -> ConnectionState;
}