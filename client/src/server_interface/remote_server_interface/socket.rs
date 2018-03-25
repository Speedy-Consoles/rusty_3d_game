use std::io;
use std::net::UdpSocket;
use std::time::Duration;

use shared::net::socket::WrappedUdpSocket;

pub struct WrappedClientUdpSocket {
    pub udp_socket: UdpSocket,
}

impl WrappedUdpSocket<()> for WrappedClientUdpSocket {
    fn send_to(&self, buf: &[u8], _addr: ()) -> io::Result<usize> {
        self.udp_socket.send(buf)
    }

    fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, ())> {
        self.udp_socket.recv(buf).map(|read| (read, ()))
    }

    fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.udp_socket.set_nonblocking(nonblocking)
    }

    fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.udp_socket.set_read_timeout(timeout)
    }
}