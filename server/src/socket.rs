use std::io;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::time::Instant;
use std::collections::HashMap;

use net2::UdpBuilder;

use shared::net::ClientMessage;
use shared::net::ServerMessage;
use shared::net::Packable;
use shared::net::MAX_MESSAGE_LENGTH;

use self::IdOrAddr::*;

pub enum IdOrAddr {
    Addr(SocketAddr),
    Id(u64),
}

impl IdOrAddr {
    pub fn identified(&self) -> bool {
        match self {
            &Addr(_) => false,
            &Id(_) => true,
        }
    }
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
        self.client_ids_by_addr.insert(addr, id);
        self.client_addrs_by_id.insert(id, addr);
    }

    pub fn remove_client(&mut self, id: u64) {
        let addr = self.client_addrs_by_id.remove(&id).unwrap();
        self.client_ids_by_addr.remove(&addr).unwrap();
    }

    pub fn send_to(&self, msg: &ServerMessage, id_or_addr: IdOrAddr) {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        let amount = msg.pack(&mut buf).unwrap();
        let addr = match id_or_addr {
            IdOrAddr::Addr(addr) => addr,
            IdOrAddr::Id(id) => *self.client_addrs_by_id.get(&id).unwrap(),
        };
        self.udp_socket.send_to(&buf[..amount], addr).unwrap();
    }

    pub fn broadcast(&self, msg: &ServerMessage) {
        for addr in self.client_ids_by_addr.keys() {
            self.send_to(msg, IdOrAddr::Addr(*addr));
        }
    }

    pub fn recv_from_until(&self, until: Instant) -> io::Result<Option<(ClientMessage, IdOrAddr)>> {
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
    fn recv_from(&self) -> io::Result<Option<(ClientMessage, IdOrAddr)>> {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        loop {
            match self.udp_socket.recv_from(&mut buf) {
                Ok((amount, addr)) => {
                    let id_or_addr = self.client_ids_by_addr.get(&addr)
                        .map_or(Addr(addr), |id| Id(*id));
                    match ClientMessage::unpack(&buf[..amount]) {
                        Ok(msg) => return Ok(Some((msg, id_or_addr))),
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