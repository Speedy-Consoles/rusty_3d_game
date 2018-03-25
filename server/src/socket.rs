use std::io;
use std::time::Duration;
use std::net::SocketAddr;
use std::net::UdpSocket;

use shared::net::socket::WrappedUdpSocket;

pub struct WrappedServerUdpSocket {
    pub udp_socket: UdpSocket,
}

impl WrappedUdpSocket<SocketAddr> for WrappedServerUdpSocket {
    fn send_to(&self, buf: &[u8], addr: SocketAddr) -> io::Result<usize> {
        self.udp_socket.send_to(buf, addr)
    }

    fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        self.udp_socket.recv_from(buf)
    }

    fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()> {
        self.udp_socket.set_nonblocking(nonblocking)
    }

    fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.udp_socket.set_read_timeout(timeout)
    }
}