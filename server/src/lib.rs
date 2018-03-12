extern crate net2;

extern crate shared;

use std::time::Instant;
use std::net::UdpSocket;
use std::net::SocketAddr;
use std::collections::HashMap;
use std::io::ErrorKind;

use net2::UdpBuilder;

use shared::consts;
use shared::consts::TICK_SPEED;
use shared::model::Model;
use shared::model::world::character::CharacterInput;
use shared::net::ClientMessage;
use shared::net::ServerMessage;
use shared::net::Packable;
use shared::net::Snapshot;
use shared::net::DisconnectReason;
use shared::net::MAX_MESSAGE_LENGTH;

struct Client {
    addr: SocketAddr,
    inputs: HashMap<u64, CharacterInput>,
    last_msg_time: Instant,
}

pub struct Server {
    socket: UdpSocket,
    model: Model,
    tick: u64,
    clients: HashMap<u64, Client>,
    clients_id_by_addr: HashMap<SocketAddr, u64>,
    to_remove_clients: HashMap<u64, DisconnectReason>,
}

impl Server {
    pub fn new() -> Server {
        Server {
            socket: UdpBuilder::new_v6().unwrap()
                .only_v6(false).unwrap()
                .bind(("::", 51946)).unwrap(),
            model: Model::new(),
            tick: 0,
            clients: HashMap::new(),
            clients_id_by_addr: HashMap::new(),
            to_remove_clients: HashMap::new(),
        }
    }

    pub fn run(&mut self) {
        // for tick rate display
        let mut last_sec = Instant::now();
        let mut tick_counter = 0;

        // for sleep timing
        let start_tick_time = Instant::now();
        let mut next_tick_time;

        // main loop
        loop { // TODO add way to exit
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
            let snapshot = ServerMessage::Snapshot(Snapshot::new(self.tick, &self.model));
            self.broadcast(snapshot);
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
                self.to_remove_clients.insert(*id, DisconnectReason::TimedOut);
            }
        }
    }

    fn remove_clients(&mut self) {
        // TODO find a way to move the reason instead of copying it
        for (&id, &reason) in self.to_remove_clients.iter() {
            let name = self.model.remove_player(id).unwrap().take_name();
            let msg = ServerMessage::PlayerDisconnect { id, name, reason };
            self.broadcast(msg);
            let client = self.clients.remove(&id).unwrap();
            self.clients_id_by_addr.remove(&client.addr).unwrap();
        }
        self.to_remove_clients.clear();
    }

    fn handle_traffic(&mut self, until: Instant) {
        loop {
            let now = Instant::now();
            if until <= now {
                break;
            }
            self.socket.set_read_timeout(Some(until - now)).unwrap();
            if let Some((msg, src)) = self.recv_from() {
                self.handle_message(msg, src);
            }
        }
    }

    fn handle_message(&mut self, message: ClientMessage, src: SocketAddr) {
        let recv_time = Instant::now();
        let id_option = self.clients_id_by_addr.get(&src).map(|id| *id);
        if let Some(id) = id_option {
            self.clients.get_mut(&id).unwrap().last_msg_time = recv_time;
        }
        match message {
            ClientMessage::ConnectionRequest => {
                if id_option.is_some() {
                    return;
                }
                let new_id = self.model.add_player(String::from("UnknownPlayer"));
                self.clients.insert(new_id, Client {
                    addr: src,
                    inputs: HashMap::new(),
                    last_msg_time: recv_time,
                });
                self.clients_id_by_addr.insert(src, new_id);
                self.send_to(ServerMessage::ConnectionConfirm(new_id), src);
            },
            ClientMessage::EchoRequest(id) => self.send_to(ServerMessage::EchoResponse(id), src),
            ClientMessage::Input { tick, input } => {
                if let Some(id) = id_option {
                    let client = self.clients.get_mut(&id).unwrap();
                    if tick > self.tick {
                        client.inputs.insert(tick, input);
                        // TODO ignore insanely high ticks
                    } else {
                        println!("Input came too late!");
                    }
                }
            },
            ClientMessage::Leave => {
                if let Some(id) = id_option {
                    self.to_remove_clients.insert(id, DisconnectReason::Disconnected);
                }
            },
        }
    }

    fn send_to(&self, msg: ServerMessage, dst: SocketAddr) {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        let amount = msg.pack(&mut buf).unwrap();
        self.socket.send_to(&buf[..amount], dst).unwrap();
    }

    fn broadcast(&self, msg: ServerMessage) {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        let amount = msg.pack(&mut buf).unwrap();
        for client in self.clients.values() {
            self.socket.send_to(&buf[..amount], client.addr).unwrap();
        }
    }

    fn recv_from(&self) -> Option<(ClientMessage, SocketAddr)> {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        match self.socket.recv_from(&mut buf) {
            Ok((amount, src)) => {
                match ClientMessage::unpack(&buf[..amount]) {
                    Ok(msg) => Some((msg, src)),
                    Err(e) => {
                        println!("{:?}", e);
                        None
                    },
                }
            },
            Err(e) => {
                match e.kind() {
                    ErrorKind::WouldBlock | ErrorKind::TimedOut => (),
                    _ => println!("{:?}", e),
                };
                None
            }
        }
    }
}