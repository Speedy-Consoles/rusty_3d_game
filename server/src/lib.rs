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
use shared::net::ClientMessage;
use shared::net::ServerMessage;
use shared::net::Snapshot;
use shared::net::ConnectionCloseReason;

use socket::Socket;
use socket::IdOrAddr;

struct Client {
    inputs: HashMap<u64, CharacterInput>,
    last_msg_time: Instant,
}

pub struct Server {
    socket: Socket,
    model: Model,
    tick: u64,
    clients: HashMap<u64, Client>,
    to_remove_clients: HashMap<u64, ConnectionCloseReason>,
    closing: bool,
}

impl Server {
    pub fn new() -> io::Result<Server> {
        Ok(Server {
            socket: Socket::new(51946)?,
            model: Model::new(),
            tick: 0,
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
        let mut next_tick_time;

        // main loop
        while !self.closing { // TODO add way to exit
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
            let msg = ServerMessage::Snapshot(Snapshot::new(self.tick, &self.model));
            self.socket.broadcast(&msg);
            next_tick_time = start_tick_time + (self.tick + 1) / TICK_SPEED;
            tick_counter += 1;

            // display tick rate
            let now = Instant::now();
            if now - last_sec > std::time::Duration::from_secs(1) {
                println!("ticks/s: {}, players: {}", tick_counter, self.clients.len());
                tick_counter = 0;
                last_sec += std::time::Duration::from_secs(1)
            }

            // sleep / handle traffic
            self.handle_traffic(next_tick_time);
            self.tick += 1;
        }
    }

    fn check_timeouts(&mut self) {
        let now = Instant::now();
        for (id, client) in self.clients.iter() {
            if now - client.last_msg_time > consts::time_out_delay() {
                self.to_remove_clients.insert(*id, ConnectionCloseReason::TimedOut);
            }
        }
    }

    fn remove_clients(&mut self) {
        // TODO find a way to move the reason instead of copying it
        for (&id, &reason) in self.to_remove_clients.iter() {
            let _name = self.model.remove_player(id).unwrap().take_name(); // TODO for leave message
            let msg = ServerMessage::ConnectionClose(reason);
            self.socket.send_to(&msg, IdOrAddr::Id(id));
            self.socket.remove_client(id);
        }
        self.to_remove_clients.clear();
    }

    fn handle_traffic(&mut self, until: Instant) {
        loop {
            match self.socket.recv_from_until(until) {
                Ok(Some((msg, src))) => self.handle_message(msg, src),
                Ok(None) => break,
                Err(e) => {
                    println!("ERROR: Network broken: {:?}", e);
                    self.closing = true;
                    let now = Instant::now();
                    if now < until {
                        thread::sleep(until - now);
                    }
                    break;
                }
            }
            // TODO maybe add conditional break here, to make sure the server continues ticking on DDoS
        }
    }

    fn handle_message(&mut self, message: ClientMessage, src: IdOrAddr) {
        let recv_time = Instant::now();
        if let IdOrAddr::Id(id) = src {
            self.clients.get_mut(&id).unwrap().last_msg_time = recv_time;
        }
        match message {
            ClientMessage::ConnectionRequest => {
                if let IdOrAddr::Addr(addr) = src {
                    let new_id = self.model.add_player(String::from("UnknownPlayer"));
                    self.clients.insert(new_id, Client {
                        inputs: HashMap::new(),
                        last_msg_time: recv_time,
                    });
                    self.socket.add_client(new_id, addr);
                    self.socket.send_to(&ServerMessage::ConnectionConfirm(new_id), src);
                }
            },
            ClientMessage::Input { tick, input } => {
                if let IdOrAddr::Id(id) = src {
                    let client = self.clients.get_mut(&id).unwrap();
                    if tick > self.tick {
                        client.inputs.insert(tick, input);
                        // TODO ignore insanely high ticks
                    } else {
                        println!(
                            "Input came too late! | Current tick: {} | Target tick: {}",
                            self.tick,
                            tick,
                        );
                    }
                }
            },
            ClientMessage::DisconnectRequest => {
                if let IdOrAddr::Id(id) = src {
                    self.to_remove_clients.insert(id, ConnectionCloseReason::UserDisconnect);
                }
            },
        }
    }
}