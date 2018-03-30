mod connected_state;
mod socket;

use std::time::Instant;
use std::io;
use std::net::UdpSocket;
use std::net::SocketAddr;
use std::thread;
use std::collections::VecDeque;

use shared::model::world::character::CharacterInput;
use shared::consts;
use shared::net::socket::SocketEvent;
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
use super::HandleTrafficResult;
use self::connected_state::ConnectedState;
use self::InternalState::*;
use self::InternalDisconnectedReason::*;
use self::socket::WrappedClientUdpSocket;

type EventQueue = VecDeque<SocketEvent<(), ServerMessage>>;

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
    Disconnecting,
    Disconnected(InternalDisconnectedReason),
}

type ClientSocket = ReliableSocket<(), ClientMessage, ServerMessage, WrappedClientUdpSocket>;

pub struct RemoteServerInterface {
    event_queue: EventQueue,
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
            event_queue: EventQueue::new(),
            socket: ReliableSocket::new(
                WrappedClientUdpSocket { udp_socket },
                consts::timeout_duration(),
                consts::disconnect_force_timeout()
            ),
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
            self.socket.terminate(con_id, &mut self.event_queue);
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
                        ConMessage::Unreliable(umsg) => con_state.handle_unreliable_message(umsg),
                    }
                }
            },
            Disconnecting { .. } | Disconnected(_) => (),
        }
    }
}

impl ServerInterface for RemoteServerInterface {
    fn do_tick(&mut self, character_input: CharacterInput) {
        match self.internal_state {
            Connecting { ref mut resend_time } => {
                *resend_time = Instant::now() + consts::connection_request_resend_interval();
                self.socket.send_to_conless((), ConnectionRequest, &mut self.event_queue);
            },
            Connected { ref mut con_state, con_id } => {
                con_state.do_tick(character_input, &mut self.socket, con_id, &mut self.event_queue)
            },
            Disconnecting | Disconnected(_) => (),
        };
    }

    fn handle_traffic(&mut self, until: Instant) -> HandleTrafficResult {
        // TODO maybe also check if the internal state fits the events?
        match self.event_queue.pop_front().or_else(|| self.socket.recv_from_until(until)) {
            Some(SocketEvent::MessageReceived(msg)) => {
                self.handle_message(msg);
                HandleTrafficResult::Interrupt
            },
            Some(SocketEvent::DoneDisconnecting(_)) => {
                println!("DEBUG: Disconnected gracefully!");
                self.internal_state = Disconnected(UserDisconnect);
                HandleTrafficResult::Interrupt
            },
            Some(SocketEvent::TimeoutDuringDisconnect { .. }) => {
                println!("DEBUG: Timed out during disconnect!");
                // TODO inform about unsent messages
                self.internal_state = Disconnected(UserDisconnect);
                HandleTrafficResult::Interrupt
            },
            Some(SocketEvent::Timeout { .. }) | Some(SocketEvent::ConReset { .. }) => {
                println!("DEBUG: Timed out!");
                // TODO inform about unsent messages
                self.internal_state = Disconnected(TimedOut);
                HandleTrafficResult::Interrupt
            },
            Some(SocketEvent::NetworkError(e)) => {
                println!("ERROR: Network broken: {:?}", e);
                self.internal_state = Disconnected(NetworkError(e));
                let now = Instant::now();
                if now < until {
                    thread::sleep(until - now);
                }
                HandleTrafficResult::Interrupt
            }
            None => HandleTrafficResult::Timeout,
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

    fn next_game_tick_time(&self) -> Option<Instant> {
        match self.internal_state {
            Connecting { resend_time } => Some(resend_time),
            Connected { ref con_state, .. } => con_state.next_tick_time(),
            Disconnecting | Disconnected(_) => None,
        }
    }

    fn disconnect(&mut self) {
        match self.internal_state {
            Connecting { .. } => {
                // TODO send some disconnection message
                self.internal_state = Disconnected(UserDisconnect);
            },
            Connected { con_id, .. } => {
                self.socket.send_to_reliable(con_id, DisconnectRequest, &mut self.event_queue);
                self.socket.disconnect(con_id);
                self.internal_state = Disconnecting;
            },
            _ => (),
        };
    }

    fn do_socket_tick(&mut self) {
        self.socket.do_tick(&mut self.event_queue);
    }

    fn next_socket_tick_time(&self) -> Option<Instant> {
        self.socket.next_tick_time()
    }
}