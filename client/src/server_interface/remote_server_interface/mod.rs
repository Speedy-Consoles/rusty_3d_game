mod after_snapshot_state;
mod network;

use std::time::Instant;
use std::io;
use std::net::SocketAddr;

use shared::model::world::character::CharacterInput;
use shared::net::ServerMessage;
use shared::net::ClientMessage;
use shared::net::Snapshot;
use shared::net::DisconnectReason;

use super::ConnectionState;
use super::ServerInterface;
use self::InternalState::*;
use self::ConnectedState::*;
use self::network::Network;
use self::after_snapshot_state::AfterSnapshotState;

enum ConnectedState {
    BeforeSnapshot {
        my_player_id: u64,
    },
    AfterSnapshot(AfterSnapshotState),
}

enum InternalState {
    Connecting,
    Connected(ConnectedState),
    Disconnecting,
    Disconnected,
}

pub struct RemoteServerInterface {
    internal_state: InternalState,
    network: Network
}

impl RemoteServerInterface {
    pub fn new(addr: SocketAddr) -> io::Result<RemoteServerInterface> {
        Ok(RemoteServerInterface {
            network: Network::new(addr)?,
            internal_state: Connecting,
        })
    }

    fn handle_message(&mut self, msg: ServerMessage) {
        use shared::net::ServerMessage::*;
        match msg {
            ConnectionConfirm(my_player_id) => self.on_connection_confirm(my_player_id),
            ConnectionClose(reason) => self.on_connection_close(reason),
            Snapshot(s) => self.on_snapshot(s),
        }
    }

    fn on_snapshot(&mut self, snapshot: Snapshot) {
        let recv_time = Instant::now();
        if let Connected(ref mut connected_state) = self.internal_state {
            match connected_state {
                &mut BeforeSnapshot { my_player_id } => *connected_state = AfterSnapshot(
                    AfterSnapshotState::new(my_player_id, snapshot, recv_time)
                ),
                &mut AfterSnapshot(ref mut after_snapshot_state) => {
                    after_snapshot_state.on_snapshot(snapshot, recv_time)
                }
            }
        }
    }

    fn on_connection_confirm(&mut self, my_player_id: u64) {
        match self.internal_state {
            Connecting => self.internal_state = Connected(BeforeSnapshot { my_player_id }),
            Connected { .. } | Disconnecting | Disconnected => (),
        }
    }

    fn on_connection_close(&mut self, reason: DisconnectReason) {
        match self.internal_state {
            Connecting | Disconnecting | Connected { .. } => {
                match reason {
                    DisconnectReason::Kicked => println!("You were kicked."),
                    DisconnectReason::TimedOut => println!("You timed out."),
                    DisconnectReason::UserDisconnect => println!("You left."),
                }
                self.internal_state = Disconnected;
            },
            Disconnected => (),
        }
    }
}

impl ServerInterface for RemoteServerInterface {
    fn do_tick(&mut self, character_input: CharacterInput) {
        match self.internal_state {
            Connected(AfterSnapshot(ref mut after_snapshot_state)) => {
                after_snapshot_state.do_tick(&self.network, character_input)
            },
            _ => (), // TODO
        }
    }

    fn handle_traffic(&mut self, until: Instant) {
        // TODO guarantee to empty the socket
        loop {
            let now = Instant::now();
            if until <= now {
                break;
            }
            if let Some(msg) = self.network.recv(Some(until - now)) {
                self.handle_message(msg);
            }
        }
    }

    fn connection_state(&self) -> ConnectionState {
        match self.internal_state {
            Connecting | Connected(BeforeSnapshot { .. }) => ConnectionState::Connecting,
            Disconnecting => ConnectionState::Disconnecting,
            Disconnected => ConnectionState::Disconnected,
            Connected(AfterSnapshot(ref state)) => state.connection_state(),
        }
    }

    fn disconnect(&mut self) {
        match self.internal_state {
            Connecting => {
                self.network.send(ClientMessage::DisconnectRequest);
                self.internal_state = Disconnected;
            },
            Connected { .. } => {
                self.network.send(ClientMessage::DisconnectRequest);
                self.internal_state = Disconnecting;
            },
            Disconnecting | Disconnected => (),
        }
    }
}