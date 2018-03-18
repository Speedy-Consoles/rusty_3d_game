mod connected_state;
mod socket;

use std::time::Instant;
use std::io;
use std::net::SocketAddr;
use std::thread;

use shared::model::world::character::CharacterInput;
use shared::consts;
use shared::net::ServerMessage;
use shared::net::ConServerMessage::*;
use shared::net::ConLessServerMessage::*;
use shared::net::ConClientMessage::*;
use shared::net::ConLessClientMessage::*;
use shared::net::ConnectionCloseReason;
use shared::net::ConnectionCloseReason::*;

use super::DisconnectedReason;
use super::ConnectionState;
use super::ServerInterface;
use self::connected_state::ConnectedState;
use self::InternalState::*;
use self::socket::ClientSocket;

enum InternalState {
    Connecting {
        resend_time: Instant,
    },
    Connected(ConnectedState),
    Disconnecting {
        force_timeout_time: Instant,
    },
    ConnectionClosed(ConnectionCloseReason),
    NetworkError(io::Error),
}

pub struct RemoteServerInterface {
    internal_state: InternalState,
    socket: ClientSocket,
}

impl RemoteServerInterface {
    pub fn new(addr: SocketAddr) -> io::Result<RemoteServerInterface> {
        let socket = ClientSocket::new(addr)?;
        Ok(RemoteServerInterface {
            socket,
            internal_state: Connecting {
                resend_time: Instant::now(),
            },
        })
    }

    fn handle_message(&mut self, message: ServerMessage) {
        if let ServerMessage::Connected(ConnectionClose(reason)) = message {
            self.internal_state = ConnectionClosed(reason);
        } else {
            match self.internal_state {
                Connecting { .. } => {
                    if let ServerMessage::Connectionless(ConnectionConfirm(my_player_id)) = message {
                        self.internal_state = Connected(ConnectedState::new(my_player_id));
                    }
                },
                Connected(ref mut state) => {
                    if let ServerMessage::Connected(msg) = message {
                        state.handle_message(msg);
                    }
                },
                Disconnecting { .. } | ConnectionClosed(_) | NetworkError(_) => (),
            }
        }
    }
}

impl ServerInterface for RemoteServerInterface {
    fn do_tick(&mut self, character_input: CharacterInput) {
        match self.internal_state {
            Connecting { ref mut resend_time } => {
                self.socket.send_connectionless(ConnectionRequest);
                *resend_time = Instant::now() + consts::connection_request_resend_interval();
            },
            Connected(ref mut state) => state.do_tick(&self.socket, character_input),
            Disconnecting { force_timeout_time } => {
                if Instant::now() > force_timeout_time {
                    self.internal_state = ConnectionClosed(TimedOut);
                }
            }
            ConnectionClosed(_) | NetworkError(_) => (),
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
            Connecting { .. } => ConnectionState::Connecting,
            Connected(ref state) => state.connection_state(),
            Disconnecting { .. } => ConnectionState::Disconnecting,
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

    fn next_tick_time(&self) -> Option<Instant> {
        match self.internal_state {
            Connecting { resend_time } => Some(resend_time),
            Connected(ref state) => state.next_tick_time(),
            Disconnecting { .. } | ConnectionClosed(_) | NetworkError(_) => None,
        }
    }

    fn disconnect(&mut self) {
        match self.internal_state {
            Connecting { .. } | Connected(_) => {
                self.socket.send_connected(DisconnectRequest);
                self.internal_state = Disconnecting {
                    force_timeout_time: Instant::now() + consts::disconnect_force_timeout()
                };
            },
            Disconnecting { .. } | ConnectionClosed(_) | NetworkError(_) => (),
        }
    }
}