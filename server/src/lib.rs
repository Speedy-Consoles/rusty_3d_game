mod socket;

extern crate net2;

extern crate shared;

use std::thread;
use std::time::Instant;
use std::collections::HashMap;
use std::io;

use shared::consts;
use shared::consts::TICK_SPEED;
use shared::model::Model;
use shared::model::world::character::CharacterInput;
use shared::tick_time::TickInstant;
use shared::net::socket::Socket;
use shared::net::ConClientMessage::*;
use shared::net::ConLessClientMessage::*;
use shared::net::ConServerMessage::*;
use shared::net::ConLessServerMessage::*;
use shared::net::Snapshot;
use shared::net::ConnectionCloseReason;

use socket::ServerSocket;
use socket::CheckedClientMessage;

struct Client {
    inputs: HashMap<u64, CharacterInput>,
    last_msg_time: Instant,
}

pub struct Server {
    socket: ServerSocket,
    model: Model,
    tick: u64,
    tick_time: Instant,
    next_tick_time: Instant,
    clients: HashMap<u64, Client>,
    to_remove_clients: HashMap<u64, ConnectionCloseReason>,
    closing: bool,
}

impl Server {
    pub fn new() -> io::Result<Server> {
        Ok(Server {
            socket: ServerSocket::new(51946)?,
            model: Model::new(),
            tick: 0,
            tick_time: Instant::now(),
            next_tick_time: Instant::now(),
            clients: HashMap::new(),
            to_remove_clients: HashMap::new(),
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
            for (id, client) in self.clients.iter_mut() {
                if let Some(input) = client.inputs.remove(&self.tick) {
                    self.model.set_character_input(*id, input);
                }
            }
            self.model.do_tick();
            let msg = SnapshotMessage(Snapshot::new(self.tick, &self.model));
            self.socket.broadcast(msg);
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
        let now = Instant::now();
        for (id, client) in self.clients.iter() {
            if now - client.last_msg_time > consts::playing_timeout() {
                self.to_remove_clients.insert(*id, ConnectionCloseReason::TimedOut);
            }
        }
    }

    fn remove_clients(&mut self) {
        // TODO find a way to move the reason instead of copying it
        for (id, reason) in self.to_remove_clients.drain() {
            let _name = self.model.remove_player(id).unwrap().take_name(); // TODO for leave message
            self.clients.remove(&id);
            let msg = ConnectionClose(reason);
            self.socket.send_to_connected(msg, id);
            self.socket.remove_client(id);
            // TODO broadcast leave message
        }
    }

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

    fn handle_message(&mut self, message: CheckedClientMessage) {
        let recv_time = Instant::now();
        match message {
            CheckedClientMessage::Connected(msg, addr) => {
                match msg {
                    ConnectionRequest => {
                        if let Some(_id) = self.socket.client_id_by_addr(addr) {
                            // TODO send another connection confirm to client and reset their connection meta data
                        } else {
                            let new_id = self.model.add_player(String::from("UnknownPlayer"));
                            self.clients.insert(new_id, Client {
                                inputs: HashMap::new(),
                                last_msg_time: recv_time,
                            });
                            self.socket.add_client(new_id, addr);
                            self.socket.send_to_connectionless(
                                ConnectionConfirm(new_id),
                                addr,
                            );
                        }
                    },
                }
            },
            CheckedClientMessage::Connectionless(msg, id) => {
                self.clients.get_mut(&id).unwrap().last_msg_time = recv_time;
                match msg {
                    InputMessage { tick, input } => {
                        let client = self.clients.get_mut(&id).unwrap();
                        if tick > self.tick {
                            self.socket.send_to_connected(
                                InputAck {
                                    input_tick: tick,
                                    arrival_tick_instant: TickInstant::from_interval(
                                        self.tick,
                                        self.tick_time,
                                        self.next_tick_time,
                                        recv_time,
                                    )
                                },
                                id,
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
                    DisconnectRequest => {
                        self.to_remove_clients.insert(id, ConnectionCloseReason::UserDisconnect);
                    },
                }
            }
        }
    }
}