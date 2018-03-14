use std::io;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::time::Instant;
use std::collections::HashMap;

use net2::UdpBuilder;

use shared::net::ClientMessage;
use shared::net::ConClientMessage;
use shared::net::ConLessClientMessage;
use shared::net::ServerMessage;
use shared::net::ConServerMessage;
use shared::net::ConLessServerMessage;
use shared::net::Packable;
use shared::net::MAX_MESSAGE_LENGTH;

use self::CheckedClientMessage::*;

pub enum CheckedClientMessage {
    Connected(ConLessClientMessage, SocketAddr),
    Connectionless(ConClientMessage, u64),
}

pub struct Socket {
    udp_socket: UdpSocket,
    client_ids_by_addr: HashMap<SocketAddr, u64>,
    client_addrs_by_id: HashMap<u64, SocketAddr>,
}

impl Socket {
    pub fn new(port: u16) -> io::Result<Socket> {
        // create IPv6 UDP socket with IPv4 compatibility
        Ok(Socket {
            udp_socket: UdpBuilder::new_v6()?.only_v6(false)?.bind(("::", port))?,
            client_ids_by_addr: HashMap::new(),
            client_addrs_by_id: HashMap::new(),
        })
    }

    pub fn add_client(&mut self, id: u64, addr: SocketAddr) {
        if self.client_ids_by_addr.insert(addr, id).is_some() {
            panic!("Tried to add client with addr already matched to client!");
        }
        self.client_addrs_by_id.insert(id, addr);
    }

    pub fn remove_client(&mut self, id: u64) {
        let addr = self.client_addrs_by_id.remove(&id).unwrap();
        self.client_ids_by_addr.remove(&addr).unwrap();
    }

    pub fn client_id_by_addr(&self, addr: SocketAddr) -> Option<u64> {
        self.client_ids_by_addr.get(&addr).map(|id| *id)
    }

    pub fn send_to_connected(&self, msg: ConServerMessage, id: u64) {
        let addr = self.client_addrs_by_id.get(&id).unwrap();
        self.send_to(&ServerMessage::Connected(msg), *addr);
    }

    pub fn send_to_connectionless(&self, msg: ConLessServerMessage, addr: SocketAddr) {
        self.send_to(&ServerMessage::Connectionless(msg), addr);
    }

    fn send_to(&self, msg: &ServerMessage, addr: SocketAddr) {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        let amount = msg.pack(&mut buf).unwrap();
        self.udp_socket.send_to(&buf[..amount], addr).unwrap();
    }

    pub fn broadcast(&self, msg: ConServerMessage) {
        let cmsg = ServerMessage::Connected(msg);
        for addr in self.client_ids_by_addr.keys() {
            self.send_to(&cmsg, *addr);
        }
    }

    pub fn recv_from_until(&self, until: Instant) -> io::Result<Option<CheckedClientMessage>> {
        // first make sure we read a message if there are any
        self.udp_socket.set_nonblocking(true).unwrap();
        let result = self.recv_from();
        self.udp_socket.set_nonblocking(false).unwrap();
        let mut option = result?;

        // if there was no message, wait for one until time out
        if option.is_none() {
            let now = Instant::now();
            if until <= now {
                return Ok(None);
            }
            self.udp_socket.set_read_timeout(Some(until - now)).unwrap();
            option = self.recv_from()?;
        }
        Ok(option)
    }

    // reads messages until there is a valid one or an error occurs
    // time out errors are transformed into None
    fn recv_from(&self) -> io::Result<Option<CheckedClientMessage>> {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        loop {
            match self.udp_socket.recv_from(&mut buf) {
                Ok((amount, addr)) => {
                    match ClientMessage::unpack(&buf[..amount]) {
                        Ok(ClientMessage::Connected(msg)) => {
                            if let Some(id) = self.client_ids_by_addr.get(&addr) {
                                return Ok(Some(Connectionless(msg, *id)));
                            } else {
                                println!("WARNING: Received connectionful message \
                                         without connection!");
                                // TODO send connection reset
                            }
                        },
                        Ok(ClientMessage::Connectionless(msg)) => {
                            return Ok(Some(Connected(msg, addr)));
                        },
                        Err(e) => println!(
                            "DEBUG: Received malformed message. Unpack error: {:?}",
                            e,
                        ),
                    }
                },
                Err(e) => {
                    match e.kind() {
                        ErrorKind::WouldBlock | ErrorKind::TimedOut => return Ok(None),
                        _ => return Err(e),
                    };
                }
            }
        }
    }
}