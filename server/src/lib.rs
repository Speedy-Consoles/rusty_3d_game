extern crate shared;

use std::time::Instant;
use std::net::UdpSocket;
use std::net::SocketAddr;
use std::collections::HashMap;
use std::io::ErrorKind;

use shared::util;
use shared::consts;
use shared::model::Model;
use shared::net::MAX_MESSAGE_LENGTH;
use shared::net::ClientMessage;
use shared::net::ServerMessage;

pub struct Server {
    socket: UdpSocket,
    model: Model,
    client_id_by_addr: HashMap<SocketAddr, u64>,
    client_addr_by_id: HashMap<u64, SocketAddr>,
    next_id: u64,
}

impl Server {
    pub fn new() -> Server {
        Server {
            socket: UdpSocket::bind("127.0.0.1:51946").unwrap(),
            model: Model::new(),
            client_id_by_addr: HashMap::new(),
            client_addr_by_id: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn run(&mut self) {
        // for tick rate display
        let mut last_sec = Instant::now();
        let mut tick_counter = 0;

        // for sleep timing
        let start_tick_time = Instant::now();
        let mut next_tick_time = Instant::now();
        let mut tick = 0;

        // main loop
        loop { // TODO add way to exit
            // tick
            let now = Instant::now();
            if now >= next_tick_time {
                // TODO apply character inputs for this tick
                self.model.tick();
                tick += 1;
                next_tick_time = start_tick_time
                        + util::mult_duration(&consts::tick_interval(), tick);
                tick_counter += 1;
            }

            // display rate
            let now = Instant::now();
            if now - last_sec > std::time::Duration::from_secs(1) {
                println!("ticks/s: {}", tick_counter);
                tick_counter = 0;
                last_sec += std::time::Duration::from_secs(1)
            }

            // sleep / handle traffic
            self.handle_traffic(next_tick_time);
        }
    }

    fn handle_traffic(&mut self, until: Instant) {
        loop {
            let now = Instant::now();
            if until <= now {
                break;
            }
            self.socket.set_read_timeout(Some(until - now)).unwrap();
            if let Some((msg, src)) = self.recv_from() {
                match msg {
                    ClientMessage::ConnectionRequest => {
                        if self.client_id_by_addr.contains_key(&src) {
                            continue; // join message from client who's already on the server
                        }
                        let id = self.next_id;
                        self.client_id_by_addr.insert(src, id);
                        self.client_addr_by_id.insert(id, src);
                        self.send_to(ServerMessage::ConnectionConfirm(id), src);
                        self.next_id += 1;
                    },
                    ClientMessage::EchoRequest(id) =>
                        self.send_to(ServerMessage::EchoResponse(id), src),
                    ClientMessage::Leave => {
                        // TODO
                    },
                }
                println!("{:?}", msg);
                // TODO
            }
        }
    }

    fn send_to(&mut self, msg: ServerMessage, dst: SocketAddr) {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        msg.pack(&mut buf).unwrap();
        self.socket.send_to(&buf, dst).unwrap();
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