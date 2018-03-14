mod playing_state;

use std::time::Instant;

use shared::model::world::character::CharacterInput;
use shared::net::ConServerMessage;

use server_interface::ConnectionState;
use server_interface::remote_server_interface::socket::Socket;

use self::ConnectedState::*;
use self::playing_state::PlayingState;

pub enum ConnectedState {
    WaitingForSnapshot {
        my_player_id: u64,
    },
    Playing(PlayingState),
}

impl ConnectedState {
    pub fn new(my_player_id: u64) -> ConnectedState {
        WaitingForSnapshot { my_player_id }
    }

    pub fn do_tick(&mut self, network: &Socket, character_input: CharacterInput) {
        match self {
            &mut WaitingForSnapshot { .. } => (),
            &mut Playing(ref mut state) => state.do_tick(network, character_input),
        }
    }

    pub fn handle_message(&mut self, msg: ConServerMessage) {
        let recv_time = Instant::now();
        match msg {
            ConServerMessage::Snapshot(snapshot) => match self {
                &mut WaitingForSnapshot { my_player_id } => {
                    *self = Playing(PlayingState::new(
                        my_player_id,
                        snapshot,
                        recv_time,
                    ));
                },
                &mut Playing(ref mut after_snapshot_state) => {
                    after_snapshot_state.on_snapshot(snapshot, recv_time);
                }
            },
            ConServerMessage::ConnectionClose(_) => (), // will not happen
        }
    }

    pub fn connection_state(&self) -> ConnectionState {
        match self {
            &WaitingForSnapshot { .. } => ConnectionState::Connecting,
            &Playing(ref state) => state.connection_state(),
        }
    }
}