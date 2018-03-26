mod connected_state;
mod socket;

use std::time::Instant;
use std::io;
use std::mem;
use std::net::UdpSocket;
use std::net::SocketAddr;
use std::thread;

use shared::model::world::character::CharacterInput;
use shared::consts;
use shared::net::socket::ConnectionData;
use shared::net::socket::ConnectionDataProvider;
use shared::net::socket::ReliableSocket;
use shared::net::ServerMessage;
use shared::net::ConlessServerMessage::*;
use shared::net::ReliableServerMessage::*;
use shared::net::ClientMessage;
use shared::net::ConlessClientMessage::*;
use shared::net::ReliableClientMessage::*;

use super::DisconnectedReason;
use super::ConnectionState;
use super::ServerInterface;
use self::connected_state::ConnectedState;
use self::InternalState::*;
use self::InternalDisconnectedReason::*;
use self::socket::WrappedClientUdpSocket;

enum InternalDisconnectedReason {
    NetworkError(io::Error),
    UserDisconnect,
    Kicked {
        kick_message: String,
    },
    TimedOut,
}

enum InternalState {
    Connecting {
        resend_time: Instant,
    },
    Connected {
        state: ConnectedState,
        con_data: ConnectionData,
    },
    Disconnecting {
        con_data: ConnectionData,
        force_timeout_time: Instant,
    },
    Disconnected(InternalDisconnectedReason),
}

struct EmptyConnectionDataProvider {}

impl ConnectionDataProvider<()> for EmptyConnectionDataProvider {
    fn con_data_mut(&mut self, _addr: ()) -> Option<&mut ConnectionData> {
        None
    }
}

impl ConnectionDataProvider<()> for InternalState {
    fn con_data_mut(&mut self, _addr: ()) -> Option<&mut ConnectionData> {
        match self {
            &mut Connected { ref mut con_data, .. } => Some(con_data),
            &mut Disconnecting { ref mut con_data, .. } => Some(con_data),
            _ => None,
        }
    }
}

type ClientSocket = ReliableSocket<ClientMessage, ServerMessage, (), WrappedClientUdpSocket>;

pub struct RemoteServerInterface {
    internal_state: InternalState,
    socket: ClientSocket,
}

impl RemoteServerInterface {
    pub fn new(addr: SocketAddr) -> io::Result<RemoteServerInterface> {
        // let the os decide over port
        let local_addr = match addr {
            SocketAddr::V4(_) => "0.0.0.0:0",
            SocketAddr::V6(_) => "[::]:0",
        };
        let udp_socket = UdpSocket::bind(local_addr)?;
        udp_socket.connect(addr)?;
        Ok(RemoteServerInterface {
            socket: ReliableSocket::new(WrappedClientUdpSocket { udp_socket }),
            internal_state: Connecting {
                resend_time: Instant::now(),
            },
        })
    }

    fn handle_message(&mut self, message: ServerMessage) {
        if let ServerMessage::Reliable(ConnectionClose) = message {
            self.internal_state = Disconnected(Kicked {
                kick_message: String::from("You were kicked for some reason"), // TODO replace with actual message
            });
        } else {
            match self.internal_state {
                Connecting { .. } => {
                    if let ServerMessage::Conless(ConnectionConfirm(my_player_id)) = message {
                        self.internal_state = Connected {
                            state: ConnectedState::new(my_player_id),
                            con_data: ConnectionData::new(),
                        };
                    }
                },
                Connected { ref mut state, .. } => {
                    match message {
                        ServerMessage::Conless(_) => (), // TODO connection reset?
                        ServerMessage::Reliable(msg) => state.handle_reliable_message(msg),
                        ServerMessage::Unreliable(msg) => state.handle_unreliable_message(msg),
                    }
                },
                Disconnecting { .. } | Disconnected(_) => (),
            }
        }
    }
}

impl ServerInterface for RemoteServerInterface {
    fn do_tick(&mut self, character_input: CharacterInput) {
        let mut result = Ok(());
        match self.internal_state {
            Connecting { ref mut resend_time } => {
                *resend_time = Instant::now() + consts::connection_request_resend_interval();
                result = self.socket.send_to_conless(ConnectionRequest, ());
            },
            Connected { ref mut state, ref mut con_data } => {
                result = state.do_tick(&self.socket, con_data, character_input)
            },
            Disconnecting { force_timeout_time, .. } => {
                if Instant::now() > force_timeout_time {
                    // TODO notify about unacked messages
                    self.internal_state = Disconnected(UserDisconnect);
                }
            },
            Disconnected(_) => (),
        };
        if let Err(err) = result {
            self.internal_state = Disconnected(NetworkError(err));
        }
    }

    fn handle_traffic(&mut self, until: Instant) {
        loop {
            match {
                match self.internal_state {
                    Connected { ref mut con_data, .. } => self.socket.recv_from_until(
                        until,
                        con_data,
                        &mut EmptyConnectionDataProvider {},
                    ),
                    Disconnecting { ref mut con_data, .. } => self.socket.recv_from_until(
                        until,
                        &mut EmptyConnectionDataProvider {},
                        con_data,
                    ),
                    _ => self.socket.recv_from_until(
                        until,
                        &mut EmptyConnectionDataProvider {},
                        &mut EmptyConnectionDataProvider {},
                    ),
                }
            } {
                Ok(Some((msg, _))) => self.handle_message(msg),
                Ok(None) => break,
                Err(e) => {
                    println!("ERROR: Network broken: {:?}", e);
                    self.internal_state = Disconnected(NetworkError(e));
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
            Connected { ref state, .. } => state.connection_state(),
            Disconnecting { .. } => ConnectionState::Disconnecting,
            Disconnected(ref reason) => ConnectionState::Disconnected(match reason {
                &UserDisconnect => DisconnectedReason::UserDisconnect,
                &Kicked { ref kick_message } => DisconnectedReason::Kicked { kick_message },
                &TimedOut => DisconnectedReason::TimedOut,
                &NetworkError(_) => DisconnectedReason::NetworkError,
            }),
        }
    }

    fn next_tick_time(&self) -> Option<Instant> {
        match self.internal_state {
            Connecting { resend_time } => Some(resend_time),
            Connected { ref state, .. } => state.next_tick_time(),
            Disconnecting { force_timeout_time, .. } => Some(force_timeout_time),
            Disconnected(_) => None,
        }
    }

    fn disconnect(&mut self) {
        let con_data = match self.internal_state {
            Connecting { .. } => ConnectionData::new(),
            Connected { ref mut con_data, .. } => {
                mem::replace(con_data, ConnectionData::new())
            },
            _ => return,
        };
        self.internal_state = Disconnecting {
            con_data,
            force_timeout_time: Instant::now() + consts::disconnect_force_timeout()
        };
        let result = if let Disconnecting { ref mut con_data, .. } = self.internal_state {
            self.socket.send_to_reliable(DisconnectRequest, (), con_data)
        } else {
            unreachable!()
        };
        if let Err(err) = result {
            self.internal_state = Disconnected(NetworkError(err));
        }
    }
}