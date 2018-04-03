use std::io;
use std::net::UdpSocket;
use std::net::SocketAddr;
use std::time::Instant;
use std::time::Duration;
use std::collections::BinaryHeap;
use std::iter;
use std::cmp::Reverse;

use rand;
use rand::distributions::IndependentSample;
use rand::distributions::Gamma;
use rand::Rng;

use arrayvec::ArrayVec;

use shared::util;
use shared::net::MAX_MESSAGE_LENGTH;
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
    send_distribution: Gamma,
    recv_distribution: Gamma,
    read_timeout: Option<Duration>,
    received_messages: BinaryHeap<(Reverse<Instant>, ArrayVec<[u8; MAX_MESSAGE_LENGTH]>)>,
    sent_messages: BinaryHeap<(Reverse<Instant>, ArrayVec<[u8; MAX_MESSAGE_LENGTH]>)>,
}

impl CrapNetSocket {
    pub fn new(
        addr: SocketAddr,
        send_delay_mean: f64,
        send_delay_std_dev: f64,
        send_drop_chance: f64,
        recv_delay_mean: f64,
        recv_delay_std_dev: f64,
        recv_drop_chance: f64,
    ) -> io::Result<CrapNetSocket> {
        let send_delay_shape = send_delay_mean * send_delay_mean
                / (send_delay_std_dev * send_delay_std_dev);
        let send_delay_scale = send_delay_std_dev * send_delay_std_dev / send_delay_mean;
        let recv_delay_shape = recv_delay_mean * recv_delay_mean
                / (recv_delay_std_dev * recv_delay_std_dev);
        let recv_delay_scale = recv_delay_std_dev * recv_delay_std_dev / recv_delay_mean;
        Ok(CrapNetSocket {
            socket: ConnectedSocket::new(addr)?,
            send_drop_chance,
            recv_drop_chance,
            send_distribution: Gamma::new(send_delay_shape, send_delay_scale),
            recv_distribution: Gamma::new(recv_delay_shape, recv_delay_scale),
            read_timeout: None,
            received_messages: BinaryHeap::new(),
            sent_messages: BinaryHeap::new(),
        })
    }
}

impl WrappedUdpSocket<()> for CrapNetSocket {
    fn send_to(&mut self, buf: &[u8], _addr: ()) -> io::Result<usize> {
        let mut rng = rand::thread_rng();
        if rng.gen_range(0.0, 1.0) > self.send_drop_chance {
            let delay = util::duration_from_float(self.send_distribution.ind_sample(&mut rng));
            let mut data: ArrayVec<[u8; MAX_MESSAGE_LENGTH]> = iter::repeat(0).collect();
            data.truncate(buf.len());
            data.copy_from_slice(&buf);
            self.sent_messages.push((Reverse(Instant::now() + delay), data));
        }
        Ok(buf.len())
    }

    fn recv_from(&mut self, buf: &mut [u8]) -> io::Result<(usize, ())> {
        let mut rng = rand::thread_rng();
        let until = self.read_timeout.map(|t| Instant::now() + t);
        loop {
            let now = Instant::now();
            // first check, if there are pending messages to send/receive
            let mut send = false;
            if let Some(send_msg) = self.sent_messages.peek() {
                if (send_msg.0).0 < now {
                    send = true;
                }
            }
            if send {
                let send_msg = self.sent_messages.pop().unwrap();
                self.socket.send_to(&send_msg.1, ())?;
            }
            let mut receive = false;
            if let Some(recv_msg) = self.received_messages.peek() {
                if (recv_msg.0).0 < now {
                    receive = true;
                }
            }
            if receive {
                let recv_msg = self.received_messages.pop().unwrap();
                let recv_bytes = recv_msg.1.len();
                buf[..recv_bytes].copy_from_slice(&recv_msg.1);
                return Ok((recv_bytes, ()));
            }

            // now adjust the timeout
            let mut next_action = until;
            if let Some(sent_msg) = self.sent_messages.peek() {
                if let Some(na) = next_action {
                    if (sent_msg.0).0 < na {
                        next_action = Some((sent_msg.0).0);
                    }
                } else {
                    next_action = Some((sent_msg.0).0);
                }
            }
            if let Some(recv_msg) = self.received_messages.peek() {
                if let Some(na) = next_action {
                    if (recv_msg.0).0 < na {
                        next_action = Some((recv_msg.0).0);
                    }
                } else {
                    next_action = Some((recv_msg.0).0);
                }
            }
            if let Some(na) = next_action {
                if na > now {
                    self.socket.set_read_timeout(Some(na - now))?;
                }
            }

            // do the actual receiving
            let mut data: ArrayVec<[u8; MAX_MESSAGE_LENGTH]> = iter::repeat(0).collect();
            let (read_bytes, _) = self.socket.recv_from(&mut data)?;
            if rng.gen_range(0.0, 1.0) > self.recv_drop_chance {
                let delay = util::duration_from_float(
                    self.recv_distribution.ind_sample(&mut rng)
                );
                data.truncate(read_bytes);
                self.received_messages.push((Reverse(Instant::now() + delay), data));
            }
        }
    }

    fn set_nonblocking(&mut self, nonblocking: bool) -> io::Result<()> {
        self.socket.set_nonblocking(nonblocking)
    }

    fn set_read_timeout(&mut self, timeout: Option<Duration>) -> io::Result<()> {
        self.read_timeout = timeout;
        self.socket.set_read_timeout(timeout)
    }
}