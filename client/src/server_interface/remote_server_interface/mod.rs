mod connected_state;
mod socket;

use std::time::Instant;
use std::io;
use std::net::SocketAddr;
use std::thread;

use shared::model::world::character::CharacterInput;
use shared::consts;
use shared::net::socket::ConnectionEndReason;
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
use self::socket::ConnectedSocket;
use self::socket::CrapNetSocket;

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

type ClientSocket = ReliableSocket<(), ClientMessage, ServerMessage, ConnectedSocket>;
//type ClientSocket = ReliableSocket<(), ClientMessage, ServerMessage, CrapNetSocket>;

pub struct RemoteServerInterface {
    internal_state: InternalState,
    socket: ClientSocket,
}

impl RemoteServerInterface {
    pub fn new(addr: SocketAddr) -> io::Result<RemoteServerInterface> {
        Ok(RemoteServerInterface {
            socket: ReliableSocket::new(
                ConnectedSocket::new(addr)?,
                //CrapNetSocket::new(addr, 0.5, 0.3, 0.3, 0.5, 0.3, 0.3)?,
                consts::timeout_duration(),
                consts::disconnect_force_timeout(),
                false,
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
                self.socket.send_to_conless((), ConnectionRequest);
            },
            Connected { ref mut con_state, con_id } => {
                con_state.do_tick(character_input, &mut self.socket, con_id)
            },
            Disconnecting | Disconnected(_) => (),
        };
    }

    fn handle_traffic(&mut self, until: Instant) -> HandleTrafficResult {
        // TODO maybe also check if the internal state fits the events?
        match self.socket.wait_event(until) {
            Some(SocketEvent::MessageReceived(msg)) => {
                self.handle_message(msg);
                HandleTrafficResult::Interrupt
            },
            Some(SocketEvent::DoneDisconnecting(_)) => {
                println!("DEBUG: Disconnected gracefully!");
                self.internal_state = Disconnected(UserDisconnect);
                HandleTrafficResult::Interrupt
            },
            Some(SocketEvent::ConnectionEnd { reason, .. })  => {
                match reason {
                    ConnectionEndReason::TimedOut => {
                        println!("DEBUG: Timed out!");
                    },
                    ConnectionEndReason::Reset => {
                        println!("DEBUG: Connection reset!");
                    },
                }
                // TODO inform about unsent messages
                self.internal_state = Disconnected(TimedOut);
                HandleTrafficResult::Interrupt
            },
            Some(SocketEvent::DisconnectingConnectionEnd { reason, .. }) => {
                match reason {
                    ConnectionEndReason::TimedOut => {
                        println!("DEBUG: Timed out during disconnect!");
                    },
                    ConnectionEndReason::Reset => {
                        println!("DEBUG: Connection reset during disconnect!");
                    },
                }
                // TODO inform about unsent messages
                self.internal_state = Disconnected(UserDisconnect);
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
            Disconnecting => ConnectionState::Disconnecting,
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
                self.socket.send_to_conless((), ConnectionAbort);
                self.internal_state = Disconnected(UserDisconnect);
            },
            Connected { con_id, .. } => {
                self.socket.send_to_reliable(con_id, DisconnectRequest);
                self.socket.disconnect(con_id);
                self.internal_state = Disconnecting;
            },
            _ => (),
        };
    }

    fn do_socket_tick(&mut self) {
        self.socket.do_tick();
    }

    fn next_socket_tick_time(&self) -> Option<Instant> {
        self.socket.next_tick_time()
    }
}