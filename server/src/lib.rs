mod socket;

extern crate net2;

extern crate shared;

use std::thread;
use std::time::Instant;
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;

use net2::UdpBuilder;

use shared::consts;
use shared::consts::TICK_SPEED;
use shared::consts::MAX_INPUT_TICK_LEAD;
use shared::model::Model;
use shared::model::world::character::CharacterInput;
use shared::tick_time::TickInstant;
use shared::net::socket::ConnectionEndReason;
use shared::net::socket::Event;
use shared::net::socket::ConId;
use shared::net::socket::CheckedMessage;
use shared::net::socket::ConMessage;
use shared::net::socket::ReliableSocket;
use shared::net::ClientMessage;
use shared::net::ConlessClientMessage::*;
use shared::net::ReliableClientMessage::*;
use shared::net::UnreliableClientMessage::*;
use shared::net::ServerMessage;
use shared::net::ConlessServerMessage::*;
use shared::net::UnreliableServerMessage::*;
use shared::net::Snapshot;

use socket::WrappedServerUdpSocket;
use TickTarget::*;

enum TickTarget {
    GameTick,
    SocketTick,
}

#[derive(Debug)]
struct Client {
    player_id: u64,
    inputs: HashMap<u64, CharacterInput>,
    last_input_time: Instant,
}

pub struct Server {
    socket: ReliableSocket<SocketAddr, ServerMessage, ClientMessage, WrappedServerUdpSocket>,
    clients: HashMap<ConId, Client>, // TODO consider making this an array
    client_remove_buffer: Vec<ConId>, // TODO add remove reason for message
    model: Model,
    tick: u64,
    tick_time: Instant,
    next_tick_time: Instant,
    con_id_by_player_id: HashMap<u64, ConId>,
    closing: bool,
}

impl Server {
    pub fn new() -> io::Result<Server> {
        // create IPv6 UDP socket with IPv4 compatibility
        let wrapped_socket = WrappedServerUdpSocket {
            udp_socket: UdpBuilder::new_v6()?.only_v6(false)?.bind(("::", 51946))?,
        };
        Ok(Server {
            socket: ReliableSocket::new(
                wrapped_socket,
                consts::ack_timeout_duration(),
                consts::ack_timeout_duration(),
                true,
            ),
            clients: HashMap::new(),
            client_remove_buffer: Vec::new(),
            model: Model::new(),
            tick: 0,
            tick_time: Instant::now(),
            next_tick_time: Instant::now(),
            con_id_by_player_id: HashMap::new(),
            closing: false,
        })
    }

    pub fn run(&mut self) {
        // for tick rate display
        let mut last_sec = Instant::now();
        let mut tick_counter = 0;

        // for sleep timing
        let start_tick_time = Instant::now();
        self.next_tick_time = Instant::now();

        // main loop
        while !self.closing { // TODO add way to exit
            // check input timeouts
            self.check_input_timeouts();

            // socket tick
            if let Some(next_socket_tick_time) = self.socket.next_tick_time() {
                let before_tick = Instant::now();
                if next_socket_tick_time <= before_tick {
                    self.socket.do_tick();
                }
            }

            // game tick
            let before_tick = Instant::now();
            if self.next_tick_time <= before_tick {
                // update tick
                self.tick += 1;

                // update tick times
                self.tick_time = self.next_tick_time;
                self.next_tick_time = start_tick_time + (self.tick + 1) / TICK_SPEED;

                // tick
                for (_, client) in self.clients.iter_mut() {
                    if let Some(input) = client.inputs.remove(&self.tick) {
                        self.model.set_character_input(client.player_id, input);
                    }
                }
                self.model.do_tick();
                let msg = SnapshotMessage(Snapshot::new(self.tick, &self.model));
                self.socket.broadcast_unreliable(msg);
                tick_counter += 1;

                // display tick rate
                let now = Instant::now();
                if now - last_sec > std::time::Duration::from_secs(1) {
                    println!("ticks/s: {}, players: {}", tick_counter, self.clients.len());
                    tick_counter = 0;
                    last_sec += std::time::Duration::from_secs(1)
                }
            }

            // sleep / handle traffic
            self.handle_traffic();
        }
    }

    fn handle_traffic(&mut self) -> TickTarget {
        loop {
            let mut next_loop_time = self.next_tick_time;
            let mut tick_target = GameTick;
            match self.socket.next_tick_time() {
                Some(next_socket_tick_time) if next_socket_tick_time < next_loop_time => {
                    next_loop_time = next_socket_tick_time;
                    tick_target = SocketTick;
                }
                _ => (),
            }
            match self.socket.wait_event(next_loop_time) {
                Some(Event::MessageReceived(msg)) => self.handle_message(msg),
                Some(Event::DoneDisconnecting(con_id)) => {
                    println!("DEBUG: {} disconnected gracefully!", con_id);
                }
                Some(Event::ConnectionEnd { reason, con_id }) => {
                    match reason {
                        ConnectionEndReason::TimedOut => {
                            println!("DEBUG: {} timed out!", con_id);
                        },
                        ConnectionEndReason::Reset => {
                            println!("DEBUG: {} sent connection reset!", con_id);
                        },
                    }
                    self.remove_client(con_id)
                },
                Some(Event::DisconnectingConnectionEnd { reason, con_id }) => {
                    match reason {
                        ConnectionEndReason::TimedOut => {
                            println!("DEBUG: {} timed out during disconnect!", con_id);
                        },
                        ConnectionEndReason::Reset => {
                            println!("DEBUG: {} sent connection reset during disconnect!", con_id);
                        },
                    }
                },
                Some(Event::NetworkError(e)) => {
                    println!("ERROR: Network broken: {:?}", e);
                    self.closing = true;
                    let now = Instant::now();
                    if now < self.next_tick_time {
                        thread::sleep(self.next_tick_time - now);
                    }
                    return tick_target; // this is not actually true, but we're closing anyway
                }
                None => return tick_target,
            }
            // TODO maybe add conditional break here, to make sure the server continues ticking on DDoS
        }
    }

    fn handle_message(&mut self, msg: CheckedMessage<SocketAddr, ClientMessage>) {
        if let CheckedMessage::Conful {
            con_id,
            cmsg: ConMessage::Reliable(DisconnectRequest)
        } = msg {
            self.remove_client(con_id);
            self.socket.terminate(con_id);
            return;
        }
        let recv_time = Instant::now();
        match msg {
            CheckedMessage::Conless { addr, con_id, clmsg } => {
                match clmsg {
                    ConnectionRequest => {
                        let player_id = match con_id {
                            Some(con_id) => {
                                // repeat confirm message
                                // TODO what if the connection request is different from the first one?
                                self.clients.get(&con_id).unwrap().player_id
                            },
                            None => {
                                // create new player
                                let player_id = self.model.add_player(
                                    String::from("UnknownPlayer")
                                );
                                let con_id = self.socket.connect(addr);
                                self.con_id_by_player_id.insert(player_id, con_id);
                                self.clients.insert(con_id, Client {
                                    player_id,
                                    inputs: HashMap::new(),
                                    last_input_time: recv_time,
                                });
                                // TODO broadcast join message
                                player_id
                            },
                        };
                        self.socket.send_to_conless(addr, ConnectionAccept(player_id));
                    },
                    ConnectionAbort => {
                        if let Some(con_id) = con_id {
                            self.remove_client(con_id);
                        }
                    },
                }
            },
            CheckedMessage::Conful { con_id, cmsg } => {
                let client = self.clients.get_mut(&con_id).unwrap();
                match cmsg {
                    ConMessage::Reliable(rmsg) => {
                        match rmsg {
                            DisconnectRequest => (), // handled earlier
                        }
                    },
                    ConMessage::Unreliable(umsg) => {
                        match umsg {
                            InputMessage { tick, input } => {
                                client.last_input_time = recv_time;
                                if tick <= self.tick {
                                    println!(
                                        "Input came too late! | Current tick: {} | Target tick: {}",
                                        self.tick,
                                        tick,
                                    );
                                } else if tick > self.tick + MAX_INPUT_TICK_LEAD {
                                    println!(
                                        "Input tick too advanced! | Current tick: {} \
                                         | Target tick: {}",
                                        self.tick,
                                        tick,
                                    );
                                } else {
                                    client.inputs.insert(tick, input);
                                }
                                self.socket.send_to_unreliable(
                                    con_id,
                                    InputAck {
                                        input_tick: tick,
                                        arrival_tick_instant: TickInstant::from_interval(
                                            self.tick,
                                            self.tick_time,
                                            self.next_tick_time,
                                            recv_time,
                                        )
                                    },
                                );
                            },
                        }
                    },
                }
            },
        }
    }

    fn check_input_timeouts(&mut self) {
        let now = Instant::now();
        for (&con_id, client) in self.clients.iter() {
            if now > client.last_input_time + consts::input_timeout_duration() {
                self.socket.send_to_unreliable(con_id, TimeOutMessage);
                self.socket.terminate(con_id);
                self.client_remove_buffer.push(con_id);
            }
        }
        self.remove_clients();
    }

    fn remove_client(&mut self, con_id: ConId) {
        self.client_remove_buffer.push(con_id);
        self.remove_clients();
    }

    fn remove_clients(&mut self) {
        for con_id in self.client_remove_buffer.drain(..) {
            let client = self.clients.remove(&con_id).unwrap();
            self.con_id_by_player_id.remove(&client.player_id).unwrap();
            self.model.remove_player(client.player_id);
            // TODO broadcast leave message
        }
    }
}