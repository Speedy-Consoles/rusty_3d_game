use std::io;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::time::Duration;
use std::collections::HashMap;

use net2::UdpBuilder;

use shared::net::socket::Socket;
use shared::net::ClientMessage;
use shared::net::ConClientMessage;
use shared::net::ConLessClientMessage;
use shared::net::ServerMessage;
use shared::net::ConServerMessage;
use shared::net::ConLessServerMessage;

use self::CheckedClientMessage::*;

pub enum CheckedClientMessage {
    Connected(ConLessClientMessage, SocketAddr),
    Connectionless(ConClientMessage, u64),
}

pub struct ServerSocket {
    udp_socket: UdpSocket,
    client_ids_by_addr: HashMap<SocketAddr, u64>,
    client_addrs_by_id: HashMap<u64, SocketAddr>,
}

impl ServerSocket {
    pub fn new(port: u16) -> io::Result<ServerSocket> {
        // create IPv6 UDP socket with IPv4 compatibility
        Ok(ServerSocket {
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

    pub fn broadcast(&self, msg: ConServerMessage) {
        let cmsg = ServerMessage::Connected(msg);
        for addr in self.client_ids_by_addr.keys() {
            self.send_to(&cmsg, *addr);
        }
    }
}

impl Socket<ServerMessage, ClientMessage, SocketAddr, CheckedClientMessage> for ServerSocket {
    fn send_impl(&self, buf: &[u8], addr: SocketAddr) -> io::Result<usize> {
        self.udp_socket.send_to(buf, addr)
    }

    fn recv_impl(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.udp_socket.recv_from(buf)
    }

    fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.udp_socket.set_nonblocking(nonblocking)
    }

    fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.udp_socket.set_read_timeout(timeout)
    }

    fn check_msg(&self, msg: ClientMessage, addr: SocketAddr) -> Option<CheckedClientMessage> {
        match msg {
            ClientMessage::Connected(msg) => {
                if let Some(id) = self.client_ids_by_addr.get(&addr) {
                    Some(Connectionless(msg, *id))
                } else {
                    println!("WARNING: Received connectionful message without connection!");
                    // TODO send connection reset
                    None
                }
            },
            ClientMessage::Connectionless(msg) => {
                Some(Connected(msg, addr))
            },
        }
    }
}