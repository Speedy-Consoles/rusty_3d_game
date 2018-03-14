mod after_snapshot_state;

use std::time::Instant;

use shared::model::world::character::CharacterInput;
use shared::net::ConnectedServerMessage;

use server_interface::ConnectionState;
use server_interface::remote_server_interface::socket::Socket;

use self::ConnectedState::*;
use self::after_snapshot_state::AfterSnapshotState;

pub enum ConnectedState {
    BeforeSnapshot {
        my_player_id: u64,
    },
    AfterSnapshot(AfterSnapshotState),
}

impl ConnectedState {
    pub fn new(my_player_id: u64) -> ConnectedState {
        BeforeSnapshot { my_player_id }
    }

    pub fn do_tick(&mut self, network: &Socket, character_input: CharacterInput) {
        match self {
            &mut BeforeSnapshot { .. } => (),
            &mut AfterSnapshot(ref mut state) => state.do_tick(network, character_input),
        }
    }

    pub fn handle_message(&mut self, msg: ConnectedServerMessage) {
        let recv_time = Instant::now();
        match msg {
            ConnectedServerMessage::Snapshot(snapshot) => match self {
                &mut BeforeSnapshot { my_player_id } => {
                    *self = AfterSnapshot(AfterSnapshotState::new(
                        my_player_id,
                        snapshot,
                        recv_time,
                    ));
                },
                &mut AfterSnapshot(ref mut after_snapshot_state) => {
                    after_snapshot_state.on_snapshot(snapshot, recv_time);
                }
            },
            ConnectedServerMessage::ConnectionClose(_) => (), // will not happen
        }
    }

    pub fn connection_state(&self) -> ConnectionState {
        match self {
            &BeforeSnapshot { .. } => ConnectionState::Connecting,
            &AfterSnapshot(ref state) => state.connection_state(),
        }
    }
}