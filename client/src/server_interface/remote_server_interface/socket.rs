use std::io;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::time::Instant;
use std::time::Duration;

use shared::net::socket::Socket;
use shared::net::ServerMessage;
use shared::net::ClientMessage;
use shared::net::ConClientMessage;
use shared::net::ConLessClientMessage;

pub struct ClientSocket {
    udp_socket: UdpSocket,
}

impl ClientSocket {
    pub fn new(addr: SocketAddr) -> io::Result<ClientSocket> {
        // let the os decide over port
        let local_addr = match addr {
            SocketAddr::V4(_) => "0.0.0.0:0",
            SocketAddr::V6(_) => "[::]:0",
        };
        let udp_socket = UdpSocket::bind(local_addr)?;
        udp_socket.connect(addr)?;
        let socket = ClientSocket {
            udp_socket,
        };
        Ok(socket)
    }

    pub fn send_connected(&self, msg: ConClientMessage) {
        self.send_to(&ClientMessage::Connected(msg), ());
    }

    pub fn send_connectionless(&self, msg: ConLessClientMessage) {
        self.send_to(&ClientMessage::Connectionless(msg), ());
    }

    pub fn recv_until(&self, until: Instant) -> io::Result<Option<(ServerMessage)>> {
        self.recv_from_until(until)
    }
}

impl Socket<ClientMessage, ServerMessage, (), ServerMessage> for ClientSocket {
    fn send_impl(&self, buf: &[u8], _addr: ()) -> io::Result<usize> {
        self.udp_socket.send(buf)
    }

    fn recv_impl(&self, buf: &mut [u8]) -> io::Result<(usize, ())> {
        self.udp_socket.recv(buf).map(|read| (read, ()))
    }

    fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.udp_socket.set_nonblocking(nonblocking)
    }

    fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.udp_socket.set_read_timeout(timeout)
    }

    fn check_msg(&self, msg: ServerMessage, _addr: ()) -> Option<ServerMessage> {
        Some(msg)
    }
}