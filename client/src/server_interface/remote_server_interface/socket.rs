use std::io;
use std::net::UdpSocket;
use std::net::SocketAddr;
use std::time::Instant;
use std::time::Duration;

use rand;
use rand::distributions::IndependentSample;
use rand::distributions::Gamma;
use rand::Rng;

use shared::net::socket::WrappedUdpSocket;

pub struct ConnectedSocket {
    pub socket: UdpSocket,
}

impl ConnectedSocket {
    pub fn new(addr: SocketAddr) -> io::Result<ConnectedSocket> {
        // let the os decide over port
        let local_addr = match addr {
            SocketAddr::V4(_) => "0.0.0.0:0",
            SocketAddr::V6(_) => "[::]:0",
        };
        let udp_socket = UdpSocket::bind(local_addr)?;
        udp_socket.connect(addr)?;
        Ok(ConnectedSocket { socket: udp_socket })
    }
}

impl WrappedUdpSocket<()> for ConnectedSocket {
    fn send_to(&mut self, buf: &[u8], _addr: ()) -> io::Result<usize> {
        self.socket.send(buf)
    }

    fn recv_from(&mut self, buf: &mut [u8]) -> io::Result<(usize, ())> {
        self.socket.recv(buf).map(|read| (read, ()))
    }

    fn set_nonblocking(&mut self, nonblocking: bool) -> io::Result<()> {
        self.socket.set_nonblocking(nonblocking)
    }

    fn set_read_timeout(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        self.socket.set_read_timeout(timeout)
    }
}

pub struct CrapNetSocket {
    socket: ConnectedSocket,
    send_drop_chance: f64,
    recv_drop_chance: f64,
    distribution: Gamma,
    read_timeout: Option<Duration>,
}

impl CrapNetSocket {
    pub fn new(
        addr: SocketAddr,
        send_drop_chance: f64,
        recv_drop_chance: f64
    ) -> io::Result<CrapNetSocket> {
        Ok(CrapNetSocket {
            socket: ConnectedSocket::new(addr)?,
            send_drop_chance,
            recv_drop_chance,
            distribution: Gamma::new(0.5, 0.5), // TODO
            read_timeout: None,
        })
    }
}

impl WrappedUdpSocket<()> for CrapNetSocket {
    fn send_to(&mut self, buf: &[u8], addr: ()) -> io::Result<usize> {
        // TODO delay messages
        let mut rng = rand::thread_rng();
        if rng.gen_range(0.0, 1.0) > self.send_drop_chance {
            self.socket.send_to(buf, addr)?;
        }
        Ok(buf.len())
    }

    fn recv_from(&mut self, buf: &mut [u8]) -> io::Result<(usize, ())> {
        // TODO delay messages
        let mut rng = rand::thread_rng();
        let mut result;
        let until = self.read_timeout.map(|t| Instant::now() + t);
        loop {
            result = self.socket.recv_from(buf);
            if result.is_err() {
                break;
            } else if rng.gen_range(0.0, 1.0) > self.recv_drop_chance {
                break;
            } else if let Some(until) = until {
                let now = Instant::now();
                if until > now {
                    self.socket.set_read_timeout(Some(until - now))?;
                }
            }
        }
        self.socket.set_read_timeout(self.read_timeout)?;

        result
    }

    fn set_nonblocking(&mut self, nonblocking: bool) -> io::Result<()> {
        self.socket.set_nonblocking(nonblocking)
    }

    fn set_read_timeout(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        self.read_timeout = timeout;
        self.socket.set_read_timeout(timeout)
    }
}