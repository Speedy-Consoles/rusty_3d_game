mod after_snapshot_state;
mod network;

use std::time::Instant;
use std::io;
use std::net::SocketAddr;
use std::thread;

use shared::model::world::character::CharacterInput;
use shared::net::ServerMessage;
use shared::net::ClientMessage;
use shared::net::Snapshot;
use shared::net::ConnectionCloseReason;
use shared::net::ConnectionCloseReason::*;

use super::DisconnectedReason;
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
    ConnectionRequestSent,
    Connected(ConnectedState),
    LeaveSent,
    ConnectionClosed(ConnectionCloseReason),
    NetworkError(io::Error),
}

pub struct RemoteServerInterface {
    internal_state: InternalState,
    network: Network
}

impl RemoteServerInterface {
    pub fn new(addr: SocketAddr) -> io::Result<RemoteServerInterface> {
        Ok(RemoteServerInterface {
            network: Network::new(addr)?,
            internal_state: ConnectionRequestSent,
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
            ConnectionRequestSent => {
                self.internal_state = Connected(BeforeSnapshot { my_player_id })
            },
            Connected { .. } | LeaveSent | ConnectionClosed(_) | NetworkError(_) => (),
        }
    }

    fn on_connection_close(&mut self, reason: ConnectionCloseReason) {
        match self.internal_state {
            ConnectionRequestSent | LeaveSent | Connected { .. } => {
                self.internal_state = ConnectionClosed(reason);
            },
            ConnectionClosed(_) | NetworkError(_) => (),
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
        loop {
            match self.network.recv_until(until) {
                Ok(Some(msg)) => self.handle_message(msg),
                Ok(None) => break,
                Err(e) => {
                    println!("ERROR: Network broken: {:?}", e);
                    self.internal_state = NetworkError(e);
                    let now = Instant::now();
                    if now < until {
                        thread::sleep(until - now);
                    }
                    break;
                }
            }
            // TODO maybe add conditional break here, to make sure the client stays responsive on DDoS
        }
    }

    fn connection_state(&self) -> ConnectionState {
        match self.internal_state {
            ConnectionRequestSent | Connected(BeforeSnapshot { .. }) => ConnectionState::Connecting,
            LeaveSent => ConnectionState::Disconnecting,
            ConnectionClosed(reason) => ConnectionState::Disconnected(match reason {
                UserDisconnect => DisconnectedReason::UserDisconnect,
                Kicked => DisconnectedReason::Kicked {
                    kick_message: "You were kicked for some reason.", // TODO replace with actual message
                },
                TimedOut => DisconnectedReason::TimedOut,
            }),
            NetworkError(_) => ConnectionState::Disconnected(DisconnectedReason::NetworkError),
            Connected(AfterSnapshot(ref state)) => state.connection_state(),
        }
    }

    fn disconnect(&mut self) {
        match self.internal_state {
            ConnectionRequestSent => {
                self.network.send(ClientMessage::DisconnectRequest);
                // TODO wait for response?
                self.internal_state = ConnectionClosed(UserDisconnect);
            },
            Connected { .. } => {
                self.network.send(ClientMessage::DisconnectRequest);
                self.internal_state = LeaveSent;
            },
            LeaveSent | ConnectionClosed(_) | NetworkError(_) => (),
        }
    }
}