mod connected_state;
mod socket;

use std::time::Instant;
use std::io;
use std::net::UdpSocket;
use std::net::SocketAddr;
use std::thread;

use shared::model::world::character::CharacterInput;
use shared::consts;
use shared::net::socket::RecvQueue;
use shared::net::socket::RecvQueueProvider;
use shared::net::socket::ReliableSocket;
use shared::net::ServerMessage;
use shared::net::ConlessServerMessage::*;
use shared::net::ReliableServerMessage::*;
use shared::net::ClientMessage;
use shared::net::ConlessClientMessage::*;
use shared::net::ReliableClientMessage::*;
use shared::net::ConnectionCloseReason;
use shared::net::ConnectionCloseReason::*;

use super::DisconnectedReason;
use super::ConnectionState;
use super::ServerInterface;
use self::connected_state::ConnectedState;
use self::InternalState::*;
use self::socket::WrappedClientUdpSocket;

enum InternalState {
    Connecting {
        resend_time: Instant,
    },
    Connected {
        state: ConnectedState,
        recv_queue: RecvQueue,
    },
    Disconnecting {
        force_timeout_time: Instant,
    },
    ConnectionClosed(ConnectionCloseReason),
    NetworkError(io::Error),
}

impl RecvQueueProvider<()> for InternalState {
    fn recv_queue(&mut self, _addr: ()) -> Option<&mut RecvQueue> {
        match self {
            &mut Connected { ref mut recv_queue, .. } => Some(recv_queue),
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
        if let ServerMessage::Reliable(ConnectionClose(reason)) = message {
            self.internal_state = ConnectionClosed(reason);
        } else {
            match self.internal_state {
                Connecting { .. } => {
                    if let ServerMessage::Conless(ConnectionConfirm(my_player_id)) = message {
                        self.internal_state = Connected {
                            state: ConnectedState::new(my_player_id),
                            recv_queue: RecvQueue {}, // TODO
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
                Disconnecting { .. } | ConnectionClosed(_) | NetworkError(_) => (),
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
            Connected { ref mut state, .. } => {
                result = state.do_tick(&self.socket, character_input)
            },
            Disconnecting { force_timeout_time } => {
                if Instant::now() > force_timeout_time {
                    self.internal_state = ConnectionClosed(TimedOut);
                }
            }
            ConnectionClosed(_) | NetworkError(_) => (),
        };
        if let Err(err) = result {
            self.internal_state = NetworkError(err);
        }
    }

    fn handle_traffic(&mut self, until: Instant) {
        loop {
            match self.socket.recv_from_until(until, &mut self.internal_state) {
                Ok(Some((msg, _))) => self.handle_message(msg),
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
            Connected { ref state, .. } => state.connection_state(),
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
            Connected { ref state, .. } => state.next_tick_time(),
            Disconnecting { force_timeout_time } => Some(force_timeout_time),
            ConnectionClosed(_) | NetworkError(_) => None,
        }
    }

    fn disconnect(&mut self) {
        let mut result = Ok(());
        match self.internal_state {
            Connecting { .. } | Connected { .. } => {
                self.internal_state = Disconnecting {
                    force_timeout_time: Instant::now() + consts::disconnect_force_timeout()
                };
                result = self.socket.send_to_reliable(DisconnectRequest, ());
            },
            Disconnecting { .. } | ConnectionClosed(_) | NetworkError(_) => (),
        }
        if let Err(err) = result {
            self.internal_state = NetworkError(err);
        }
    }
}