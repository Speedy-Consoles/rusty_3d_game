mod connected_state;
mod socket;

use std::time::Instant;
use std::io;
use std::net::SocketAddr;
use std::thread;

use shared::model::world::character::CharacterInput;
use shared::net::ServerMessage;
use shared::net::ConServerMessage;
use shared::net::ConLessServerMessage;
use shared::net::ConClientMessage;
use shared::net::ConLessClientMessage;
use shared::net::ConnectionCloseReason;
use shared::net::ConnectionCloseReason::*;

use super::DisconnectedReason;
use super::ConnectionState;
use super::ServerInterface;
use self::connected_state::ConnectedState;
use self::InternalState::*;
use self::socket::Socket;

enum InternalState {
    ConnectionRequestSent,
    Connected(ConnectedState),
    DisconnectRequestSent,
    ConnectionClosed(ConnectionCloseReason),
    NetworkError(io::Error),
}

pub struct RemoteServerInterface {
    internal_state: InternalState,
    socket: Socket
}

impl RemoteServerInterface {
    pub fn new(addr: SocketAddr) -> io::Result<RemoteServerInterface> {
        let socket = Socket::new(addr)?;
        socket.send_connectionless(ConLessClientMessage::ConnectionRequest);
        Ok(RemoteServerInterface {
            socket,
            internal_state: ConnectionRequestSent,
        })
    }

    fn handle_message(&mut self, message: ServerMessage) {
        match message {
            ServerMessage::Connected(msg) => {
                if let ConServerMessage::ConnectionClose(reason) = msg {
                    // special case because we need to change into disconnected state here
                    self.on_connection_close(reason);
                } else if let Connected(ref mut state) = self.internal_state {
                    // the normal case
                    state.handle_message(msg);
                }
            },
            ServerMessage::Connectionless(msg) => match msg {
                ConLessServerMessage::ConnectionConfirm(my_player_id) => {
                    self.on_connection_confirm(my_player_id)
                },
            },
        }
    }

    fn on_connection_confirm(&mut self, my_player_id: u64) {
        match self.internal_state {
            ConnectionRequestSent => {
                self.internal_state = Connected(ConnectedState::new(my_player_id))
            },
            Connected(_) | DisconnectRequestSent | ConnectionClosed(_) | NetworkError(_) => (),
        }
    }

    fn on_connection_close(&mut self, reason: ConnectionCloseReason) {
        match self.internal_state {
            ConnectionRequestSent | DisconnectRequestSent | Connected(_) => {
                self.internal_state = ConnectionClosed(reason);
            },
            ConnectionClosed(_) | NetworkError(_) => (),
        }
    }
}

impl ServerInterface for RemoteServerInterface {
    fn do_tick(&mut self, character_input: CharacterInput) {
        match self.internal_state {
            Connected(ref mut state) => state.do_tick(&self.socket, character_input),
            _ => (), // TODO
        }
    }

    fn handle_traffic(&mut self, until: Instant) {
        loop {
            match self.socket.recv_until(until) {
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
            ConnectionRequestSent => ConnectionState::Connecting,
            Connected(ref state) => state.connection_state(),
            DisconnectRequestSent => ConnectionState::Disconnecting,
            ConnectionClosed(ref reason) => ConnectionState::Disconnected(match reason {
                &UserDisconnect => DisconnectedReason::UserDisconnect,
                &Kicked => DisconnectedReason::Kicked {
                    kick_message: "You were kicked for some reason.", // TODO replace with actual message
                },
                &TimedOut => DisconnectedReason::TimedOut,
            }),
            NetworkError(_) => ConnectionState::Disconnected(DisconnectedReason::NetworkError),
        }
    }

    fn disconnect(&mut self) {
        match self.internal_state {
            ConnectionRequestSent => {
                self.socket.send_connected(ConClientMessage::DisconnectRequest);
                // TODO wait for response?
                self.internal_state = ConnectionClosed(UserDisconnect);
            },
            Connected(_) => {
                self.socket.send_connected(ConClientMessage::DisconnectRequest);
                self.internal_state = DisconnectRequestSent;
            },
            DisconnectRequestSent | ConnectionClosed(_) | NetworkError(_) => (),
        }
    }
}