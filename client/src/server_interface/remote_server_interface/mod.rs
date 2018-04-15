mod connected_state;
mod socket;

use std::time::Instant;
use std::io;
use std::net::SocketAddr;
use std::thread;

use shared::model::world::character::CharacterInput;
use shared::net::socket::ConnectionEndReason;

use super::DisconnectedReason;
use super::ConnectionState;
use super::ServerInterface;
use super::HandleTrafficResult;
use self::connected_state::ConnectedState;
use self::connected_state::ConnectedStateTickResult;
use self::InternalState::*;
use self::InternalDisconnectedReason::*;
use self::socket::ClientSocket;
use self::socket::ClientSocketEvent;

enum InternalDisconnectedReason {
    NetworkError(io::Error),
    UserDisconnect,
    Kicked {
        kick_message: String,
    },
    TimedOut,
}

#[derive(Clone, Copy)]
enum DisconnectingReason {
    UserDisconnect,
    SnapshotTimeout,
    InputAckTimeout,
}

enum InternalState {
    Connecting,
    Connected(ConnectedState),
    Disconnecting(DisconnectingReason),
    Disconnected(InternalDisconnectedReason),
}

pub struct RemoteServerInterface {
    internal_state: InternalState,
    socket: ClientSocket,
}

impl RemoteServerInterface {
    pub fn new(addr: SocketAddr) -> io::Result<RemoteServerInterface> {
        Ok(RemoteServerInterface {
            socket: ClientSocket::new(addr)?,
            internal_state: Connecting,
        })
    }
}

impl ServerInterface for RemoteServerInterface {
    fn do_tick(&mut self, character_input: CharacterInput) {
        let disconnecting_reason = match self.internal_state {
            Connected(ref mut con_state) => {
                match con_state.do_tick(character_input, &mut self.socket) {
                    ConnectedStateTickResult::Ok => None,
                    ConnectedStateTickResult::SnapshotTimeout => {
                        Some(DisconnectingReason::SnapshotTimeout)
                    },
                    ConnectedStateTickResult::InputAckTimeout => {
                        Some(DisconnectingReason::InputAckTimeout)
                    },
                }
            },
            Connecting | Disconnecting(_) | Disconnected(_) => None,
        };
        if let Some(reason) = disconnecting_reason {
            self.socket.disconnect();
            self.internal_state = Disconnecting(reason);
        }
    }

    fn handle_traffic(&mut self, until: Instant) -> HandleTrafficResult {
        if let Disconnected(_) = self.internal_state {
            let now = Instant::now();
            if until <= now {
                return HandleTrafficResult::Timeout;
            }
            thread::sleep(until - now);
            return HandleTrafficResult::Timeout;
        }
        match self.socket.wait_event(until) {
            Some(ClientSocketEvent::DoneConnecting { my_player_id }) => {
                if let Connecting = self.internal_state {
                    self.internal_state = Connected(ConnectedState::new(my_player_id));
                } else {
                    panic!("Got DoneConnecting event while not connecting!");
                }
                HandleTrafficResult::Interrupt
            },
            Some(ClientSocketEvent::SnapshotReceived(snapshot)) => {
                if let Connected(ref mut con_state) = self.internal_state {
                    con_state.on_snapshot(snapshot);
                } else {
                    panic!("Got SnapshotReceived event while not connected!");
                }
                HandleTrafficResult::Interrupt
            },
            Some(ClientSocketEvent::InputAckReceived { input_tick, arrival_tick_instant }) => {
                if let Connected(ref mut con_state) = self.internal_state {
                    con_state.on_input_ack(input_tick, arrival_tick_instant);
                } else {
                    panic!("Got InputAckReceived event while not connected!");
                }
                HandleTrafficResult::Interrupt
            },
            Some(ClientSocketEvent::ConnectionClosed) => {
                if let Connected(_) = self.internal_state {
                    self.internal_state = Disconnected(Kicked {
                        // TODO replace with actual message
                        kick_message: String::from("You were kicked for some reason"),
                    });
                } else {
                    panic!("Got ConnectionClosed event while not connected!");
                }
                HandleTrafficResult::Interrupt
            }
            Some(ClientSocketEvent::DoneDisconnecting) => {
                if let Disconnecting(reason) = self.internal_state {
                    match reason {
                        DisconnectingReason::UserDisconnect => {
                            println!("DEBUG: Disconnected gracefully!");
                            self.internal_state = Disconnected(UserDisconnect);
                        },
                        DisconnectingReason::SnapshotTimeout => {
                            println!("DEBUG: Timed out!");
                            self.internal_state = Disconnected(TimedOut);
                        },
                        DisconnectingReason::InputAckTimeout => {
                            println!("DEBUG: Timed out!");
                            self.internal_state = Disconnected(TimedOut);
                        },
                    }
                } else {
                    panic!("Got DoneDisconnecting event while not disconnecting!");
                }
                HandleTrafficResult::Interrupt
            },
            Some(ClientSocketEvent::ConnectionEnd { reason, .. }) => {
                if let Connected(_) = self.internal_state {
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
                } else {
                    panic!("Got ConnectionEnd event while not connected!");
                }
                HandleTrafficResult::Interrupt
            },
            Some(ClientSocketEvent::DisconnectingConnectionEnd { reason, .. }) => {
                if let Disconnecting(internal_reason) = self.internal_state {
                    // TODO inform about unsent messages
                    match internal_reason {
                        DisconnectingReason::UserDisconnect => {
                            match reason {
                                ConnectionEndReason::TimedOut => {
                                    println!("DEBUG: Timed out during disconnect!");
                                },
                                ConnectionEndReason::Reset => {
                                    println!("DEBUG: Connection reset during disconnect!");
                                },
                            }
                            self.internal_state = Disconnected(UserDisconnect);
                        },
                        DisconnectingReason::SnapshotTimeout => {
                            self.internal_state = Disconnected(TimedOut);
                        },
                        DisconnectingReason::InputAckTimeout => {
                            self.internal_state = Disconnected(TimedOut);
                        },
                    }
                } else {
                    panic!("Got DisconnectingConnectionEnd event while not disconnecting!");
                }
                HandleTrafficResult::Interrupt
            },
            Some(ClientSocketEvent::NetworkError(e)) => {
                println!("ERROR: Network broken: {:?}", e);
                self.internal_state = Disconnected(NetworkError(e));
                let now = Instant::now();
                if now < until {
                    thread::sleep(until - now);
                }
                HandleTrafficResult::Interrupt
            },
            None => HandleTrafficResult::Timeout,
        }
    }

    fn connection_state(&self) -> ConnectionState {
        match self.internal_state {
            Connecting { .. } => ConnectionState::Connecting,
            Connected(ref con_state) => con_state.connection_state(),
            Disconnecting(_) => ConnectionState::Disconnecting,
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
            Connected(ref con_state) => con_state.next_tick_time(),
            Connecting | Disconnecting(_) | Disconnected(_) => None,
        }
    }

    fn disconnect(&mut self) {
        match self.internal_state {
            Connecting | Connected(_) => {
                self.internal_state = Disconnecting(DisconnectingReason::UserDisconnect);
                self.socket.disconnect();
            },
            Disconnecting(_) | Disconnected(_) => (),
        }
    }

    fn do_socket_tick(&mut self) {
        self.socket.do_tick();
    }

    fn next_socket_tick_time(&self) -> Option<Instant> {
        self.socket.next_tick_time()
    }
}