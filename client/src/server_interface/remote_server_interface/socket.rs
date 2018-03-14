use std::io;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::time::Instant;

use shared::net::ServerMessage;
use shared::net::ClientMessage;
use shared::net::ConClientMessage;
use shared::net::ConLessClientMessage;
use shared::net::MAX_MESSAGE_LENGTH;
use shared::net::Packable;

pub struct Socket {
    udp_socket: UdpSocket,
}

impl Socket {
    pub fn new(addr: SocketAddr) -> io::Result<Socket> {
        // let the os decide over port
        let local_addr = match addr {
            SocketAddr::V4(_) => "0.0.0.0:0",
            SocketAddr::V6(_) => "[::]:0",
        };
        let udp_socket = UdpSocket::bind(local_addr)?;
        udp_socket.connect(addr)?;
        let socket = Socket {
            udp_socket,
        };
        Ok(socket)
    }

    pub fn send_connected(&self, msg: ConClientMessage) {
        self.send(ClientMessage::Connected(msg));
    }

    pub fn send_connectionless(&self, msg: ConLessClientMessage) {
        self.send(ClientMessage::Connectionless(msg));
    }

    fn send(&self, msg: ClientMessage) {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        let amount = msg.pack(&mut buf).unwrap();
        self.udp_socket.send(&buf[..amount]).unwrap();
    }

    pub fn recv_until(&self, until: Instant) -> io::Result<Option<ServerMessage>> {
        // first make sure we read a message if there are any
        self.udp_socket.set_nonblocking(true).unwrap();
        let result = self.recv();
        self.udp_socket.set_nonblocking(false).unwrap();
        let mut option = result?;

        // if there was no message, wait for one until time out
        if option.is_none() {
            let now = Instant::now();
            if until <= now {
                return Ok(None);
            }
            self.udp_socket.set_read_timeout(Some(until - now)).unwrap();
            option = self.recv()?;
        }
        Ok(option)
    }

    // reads messages until there is a valid one or an error occurs
    // time out errors are transformed into None
    fn recv(&self) -> io::Result<Option<ServerMessage>> {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        loop {
            match self.udp_socket.recv(&mut buf) {
                Ok(amount) => {
                    match ServerMessage::unpack(&buf[..amount]) {
                        Ok(msg) => return Ok(Some(msg)),
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