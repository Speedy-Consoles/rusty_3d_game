mod socket;

extern crate net2;

extern crate shared;

use std::thread;
use std::time::Instant;
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;

use net2::UdpBuilder;

use shared::consts::TICK_SPEED;
use shared::model::Model;
use shared::model::world::character::CharacterInput;
use shared::tick_time::TickInstant;
use shared::net::socket::ConnectionData;
use shared::net::socket::ConnectionDataWrapper;
use shared::net::socket::ReliableSocket;
use shared::net::ClientMessage;
use shared::net::ConlessClientMessage::*;
use shared::net::ReliableClientMessage::*;
use shared::net::UnreliableClientMessage::*;
use shared::net::ServerMessage;
use shared::net::ConlessServerMessage::*;
use shared::net::ReliableServerMessage::*;
use shared::net::UnreliableServerMessage::*;
use shared::net::Snapshot;

use socket::WrappedServerUdpSocket;

impl ConnectionDataWrapper for Client {
    fn con_data(&self) -> &ConnectionData {
        &self.con_data
    }

    fn con_data_mut(&mut self) -> &mut ConnectionData {
        &mut self.con_data
    }
}

enum RemoveReason {
    UserDisconnect,
    Kicked,
    TimedOut,
}

struct Client {
    player_id: u64,
    inputs: HashMap<u64, CharacterInput>,
    con_data: ConnectionData,
}

pub struct Server {
    socket: ReliableSocket<ServerMessage, ClientMessage, SocketAddr, WrappedServerUdpSocket>,
    model: Model,
    tick: u64,
    tick_time: Instant,
    next_tick_time: Instant,
    clients: HashMap<SocketAddr, Client>,
    to_remove_clients: HashMap<SocketAddr, RemoveReason>,
    removed_clients: HashMap<SocketAddr, ConnectionData>,
    closing: bool,
}

impl Server {
    pub fn new() -> io::Result<Server> {
        // create IPv6 UDP socket with IPv4 compatibility
        let wrapped_socket = WrappedServerUdpSocket {
            udp_socket: UdpBuilder::new_v6()?.only_v6(false)?.bind(("::", 51946))?,
        };
        Ok(Server {
            socket: ReliableSocket::new(wrapped_socket),
            model: Model::new(),
            tick: 0,
            tick_time: Instant::now(),
            next_tick_time: Instant::now(),
            clients: HashMap::new(),
            to_remove_clients: HashMap::new(),
            removed_clients: HashMap::new(),
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
            // update tick times
            self.tick_time = self.next_tick_time;
            self.next_tick_time = start_tick_time + (self.tick + 1) / TICK_SPEED;

            // update clients
            self.check_timeouts();
            self.remove_clients();

            // tick
            for (_, client) in self.clients.iter_mut() {
                if let Some(input) = client.inputs.remove(&self.tick) {
                    self.model.set_character_input(client.player_id, input);
                }
            }
            self.model.do_tick();
            let msg = SnapshotMessage(Snapshot::new(self.tick, &self.model));
            self.socket.broadcast_unreliable(msg, self.clients.iter_mut()).unwrap(); // TODO remove unwrap
            tick_counter += 1;

            // display tick rate
            let now = Instant::now();
            if now - last_sec > std::time::Duration::from_secs(1) {
                println!("ticks/s: {}, players: {}", tick_counter, self.clients.len());
                tick_counter = 0;
                last_sec += std::time::Duration::from_secs(1)
            }

            // sleep / handle traffic
            self.handle_traffic();
            self.tick += 1;
        }
    }

    fn check_timeouts(&mut self) {
        for (id, client) in self.clients.iter() {
            if client.con_data.timed_out() {
                self.to_remove_clients.insert(*id, RemoveReason::TimedOut);
            }
        }
    }

    fn remove_clients(&mut self) {
        for (addr, reason) in self.to_remove_clients.drain() {
            let mut client = self.clients.remove(&addr).unwrap();
            let _name = self.model.remove_player(client.player_id).unwrap().take_name(); // TODO for leave message
            let msg = ConnectionClose;
            self.socket.send_to_reliable(msg, addr, &mut client.con_data).unwrap(); // TODO remove unwrap
            self.removed_clients.insert(addr, client.con_data);
            // TODO broadcast leave message
        }
    }

    fn handle_traffic(&mut self) {
        loop {
            match self.socket.recv_from_until(
                self.next_tick_time,
                &mut self.clients,
                &mut self.removed_clients,
            ) {
                Ok(Some((msg, addr))) => self.handle_message(msg, addr),
                Ok(None) => break,
                Err(e) => {
                    println!("ERROR: Network broken: {:?}", e);
                    self.closing = true;
                    let now = Instant::now();
                    if now < self.next_tick_time {
                        thread::sleep(self.next_tick_time - now);
                    }
                    break;
                }
            }
            // TODO maybe add conditional break here, to make sure the server continues ticking on DDoS
        }
    }

    fn handle_message(&mut self, message: ClientMessage, addr: SocketAddr) {
        let recv_time = Instant::now();
        match message {
            ClientMessage::Conless(msg) => {
                match msg {
                    ConnectionRequest => {
                        if let Some(client) = self.clients.get(&addr) {
                            // TODO send another connection confirm to client and reset their connection meta data
                            return;
                        }
                        let player_id = self.model.add_player(String::from("UnknownPlayer"));
                        self.clients.insert(addr, Client {
                            player_id,
                            inputs: HashMap::new(),
                            con_data: ConnectionData::new(),
                        });
                        self.socket.send_to_conless(ConnectionConfirm(player_id), addr).unwrap(); // TODO remove unwrap
                    },
                }
            },
            ClientMessage::Reliable(msg) => {
                match msg {
                    DisconnectRequest => {
                        self.to_remove_clients.insert(addr, RemoveReason::UserDisconnect);
                    },
                }
            },
            ClientMessage::Unreliable(msg) => {
                let client = self.clients.get_mut(&addr).unwrap();
                match msg {
                    InputMessage { tick, input } => {
                        if tick > self.tick {
                            self.socket.send_to_unreliable(
                                InputAck {
                                    input_tick: tick,
                                    arrival_tick_instant: TickInstant::from_interval(
                                        self.tick,
                                        self.tick_time,
                                        self.next_tick_time,
                                        recv_time,
                                    )
                                },
                                addr,
                                &mut client.con_data,
                            ).unwrap(); // TODO remove unwrap
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
    }
}