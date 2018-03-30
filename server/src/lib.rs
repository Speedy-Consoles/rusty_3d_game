mod socket;

extern crate net2;

extern crate shared;

use std::thread;
use std::time::Instant;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::io;
use std::net::SocketAddr;

use net2::UdpBuilder;

use shared::consts;
use shared::consts::TICK_SPEED;
use shared::model::Model;
use shared::model::world::character::CharacterInput;
use shared::tick_time::TickInstant;
use shared::net::socket::SocketEvent;
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

type EventQueue = VecDeque<SocketEvent<SocketAddr, ClientMessage>>;

enum TickTarget {
    GameTick,
    SocketTick,
}

#[derive(Debug)]
struct Client {
    player_id: u64,
    inputs: HashMap<u64, CharacterInput>,
}

pub struct Server {
    socket: ReliableSocket<SocketAddr, ServerMessage, ClientMessage, WrappedServerUdpSocket>,
    event_queue: EventQueue,
    clients: HashMap<ConId, Client>, // TODO consider making this an array
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
                consts::ack_timeout(),
                consts::ack_timeout(),
            ),
            event_queue: EventQueue::new(),
            clients: HashMap::new(),
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
            // socket tick
            if let Some(next_socket_tick_time) = self.socket.next_tick_time() {
                let before_tick = Instant::now();
                if next_socket_tick_time <= before_tick {
                    self.socket.do_tick(&mut self.event_queue);
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
                self.socket.broadcast_unreliable(msg, &mut self.event_queue); // TODO remove unwrap
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
            let event = self.event_queue.pop_front().or_else(
                || self.socket.recv_from_until(next_loop_time)
            );
            match event {
                Some(SocketEvent::MessageReceived(msg)) => self.handle_message(msg),
                Some(SocketEvent::Timeout { con_id }) | Some(SocketEvent::ConReset { con_id }) => {
                    println!("DEBUG: {} timed out!", con_id);
                    self.remove_client(con_id)
                },
                Some(SocketEvent::DoneDisconnecting(con_id)) => {
                    println!("DEBUG: {} disconnected gracefully!", con_id);
                }
                Some(SocketEvent::TimeoutDuringDisconnect { con_id }) => {
                    println!("DEBUG: {} timed out during disconnect!", con_id);
                },
                Some(SocketEvent::NetworkError(e)) => {
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
                                });
                                // TODO broadcast join message
                                player_id
                            },
                        };
                        self.socket.send_to_conless(
                            addr,
                            ConnectionConfirm(player_id),
                            &mut self.event_queue,
                        );
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
                    }
                    ConMessage::Unreliable(umsg) => {
                        match umsg {
                            InputMessage { tick, input } => {
                                if tick > self.tick {
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
                                        &mut self.event_queue,
                                    );
                                    // TODO ignore insanely high ticks
                                    client.inputs.insert(tick, input);
                                } else {
                                    println!(
                                        "Input came too late! | Current tick: {} | Target tick: {}",
                                        self.tick,
                                        tick,
                                    );
                                }
                            },
                        }
                    }
                }
            },
        }
    }

    fn remove_client(&mut self, con_id: ConId) { // TODO add remove reason for message
        let client = self.clients.remove(&con_id).unwrap();
        self.con_id_by_player_id.remove(&client.player_id).unwrap();
        self.model.remove_player(client.player_id);
        // TODO broadcast leave message
        return;
    }
}