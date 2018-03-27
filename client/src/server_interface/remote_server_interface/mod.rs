mod connected_state;
mod socket;

use std::time::Instant;
use std::io;
use std::net::UdpSocket;
use std::net::SocketAddr;
use std::thread;

use shared::model::world::character::CharacterInput;
use shared::consts;
use shared::net::socket::ConId;
use shared::net::socket::ReliableSocket;
use shared::net::socket::ConMessage;
use shared::net::socket::CheckedMessage;
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
        con_id: ConId,
        con_state: ConnectedState,
    },
    Disconnecting {
        force_timeout_time: Instant,
    },
    Disconnected(InternalDisconnectedReason),
}

type ClientSocket = ReliableSocket<(), ClientMessage, ServerMessage, WrappedClientUdpSocket>;

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

    fn handle_message(&mut self, msg: CheckedMessage<(), ServerMessage>) {
        if let (
            &Connected { con_id, .. },
            &CheckedMessage::Conful { cmsg: ConMessage::Reliable(ConnectionClose), .. }
        ) = (&self.internal_state, &msg) {
            self.socket.terminate(con_id);
            self.internal_state = Disconnected(Kicked {
                kick_message: String::from("You were kicked for some reason"), // TODO replace with actual message
            });
            return;
        }
        match self.internal_state {
            Connecting { .. } => {
                if let CheckedMessage::Conless {
                    clmsg: ConnectionConfirm(my_player_id),
                    ..
                } = msg {
                    let con_id = self.socket.connect(());
                    self.internal_state = Connected {
                        con_id,
                        con_state: ConnectedState::new(my_player_id),
                    };
                }
            },
            Connected { ref mut con_state, .. } => {
                match msg {
                    CheckedMessage::Conless { .. } => (), // TODO connection reset?
                    CheckedMessage::Conful { cmsg, .. } => match cmsg {
                        ConMessage::Reliable(rmsg) => con_state.handle_reliable_message(rmsg),
                        ConMessage::Unreliable(umsg)
                            => con_state.handle_unreliable_message(umsg),
                    }
                }
            },
            Disconnecting { .. } | Disconnected(_) => (),
        }
    }
}

impl ServerInterface for RemoteServerInterface {
    fn do_tick(&mut self, character_input: CharacterInput) {
        if let Err(err) = self.socket.do_tick() {
            // TODO notify about unacked messages
            self.internal_state = Disconnected(NetworkError(err));
            return;
        }

        let mut result = Ok(());
        match self.internal_state {
            Connecting { ref mut resend_time } => {
                *resend_time = Instant::now() + consts::connection_request_resend_interval();
                result = self.socket.send_to_conless((), ConnectionRequest);
            },
            Connected { ref mut con_state, con_id } => {
                result = con_state.do_tick(character_input, &mut self.socket, con_id)
            },
            Disconnecting { force_timeout_time, .. } => {
                if self.socket.done() {
                    self.internal_state = Disconnected(UserDisconnect);
                } else if Instant::now() > force_timeout_time {
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
            match self.socket.recv_from_until(until) {
                Ok(Some(msg)) => self.handle_message(msg),
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
            Connected { ref con_state, .. } => con_state.connection_state(),
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
        // TODO consider socket ticking
        match self.internal_state {
            Connecting { resend_time } => Some(resend_time),
            Connected { ref con_state, .. } => con_state.next_tick_time(),
            Disconnecting { force_timeout_time, .. } => Some(force_timeout_time),
            Disconnected(_) => None,
        }
    }

    fn disconnect(&mut self) {
        match self.internal_state {
            Connecting { .. } => {
                // TODO send some disconnection message
                self.internal_state = Disconnected(UserDisconnect);
            },
            Connected { con_id, .. } => {
                match self.socket.send_to_reliable(con_id, DisconnectRequest) {
                    Ok(()) => self.internal_state = Disconnecting {
                        force_timeout_time: Instant::now() + consts::disconnect_force_timeout()
                    },
                    Err(err) => self.internal_state = Disconnected(NetworkError(err)),
                }
            },
            _ => (),
        };
    }
}