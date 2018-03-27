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

struct Client {
    player_id: u64,
    inputs: HashMap<u64, CharacterInput>,
}

pub struct Server {
    socket: ReliableSocket<SocketAddr, ServerMessage, ClientMessage, WrappedServerUdpSocket>,
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
            clients: HashMap::new(),
            socket: ReliableSocket::new(wrapped_socket),
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
            // update tick times
            self.tick_time = self.next_tick_time;
            self.next_tick_time = start_tick_time + (self.tick + 1) / TICK_SPEED;

            // let socket tick
            if let Err(err) = self.socket.do_tick() {
                println!("ERROR: Network broken: {:?}", err);
                return;
            }

            // tick
            for (_, client) in self.clients.iter_mut() {
                if let Some(input) = client.inputs.remove(&self.tick) {
                    self.model.set_character_input(client.player_id, input);
                }
            }
            self.model.do_tick();
            let msg = SnapshotMessage(Snapshot::new(self.tick, &self.model));
            self.socket.broadcast_unreliable(msg).unwrap(); // TODO remove unwrap
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

    /*fn remove_clients(&mut self) {
        // TODO move somewhere else
        for (addr, reason) in self.to_remove_clients.drain() {
            // TODO use reason
            let mut client = self.clients.remove(&addr).unwrap();
            let _name = self.model.remove_player(client.player_id).unwrap().take_name(); // TODO for leave message
            let msg = ConnectionClose;
            self.socket.send_to_reliable(msg, addr, &mut client.con_data).unwrap(); // TODO remove unwrap
            self.removed_clients.insert(addr, client.con_data);
            // TODO broadcast leave message
        }
    }*/

    fn handle_traffic(&mut self) {
        loop {
            match self.socket.recv_from_until(self.next_tick_time) {
                Ok(Some(msg)) => self.handle_message(msg),
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

    fn handle_message(&mut self, msg: CheckedMessage<SocketAddr, ClientMessage>) {
        if let CheckedMessage::Conful {
            con_id,
            cmsg: ConMessage::Reliable(DisconnectRequest)
        } = msg {
            let client = self.clients.remove(&con_id).unwrap();
            self.con_id_by_player_id.remove(&client.player_id).unwrap();
            self.model.remove_player(client.player_id);
            self.socket.terminate(con_id);
            // TODO send leave message
            return;
        }
        let recv_time = Instant::now();
        match msg {
            CheckedMessage::Conless { addr, clmsg } => {
                match clmsg {
                    ConnectionRequest => {
                        let player_id = self.model.add_player(String::from("UnknownPlayer"));
                        self.socket.send_to_conless(addr, ConnectionConfirm(player_id)).unwrap(); // TODO remove unwrap
                        let con_id = self.socket.connect(addr);
                        self.con_id_by_player_id.insert(player_id, con_id);
                        self.clients.insert(con_id, Client {
                            player_id,
                            inputs: HashMap::new(),
                        });
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
            },
        }
    }
}