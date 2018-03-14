use std::io;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::time::Instant;

use shared::net::ServerMessage;
use shared::net::ClientMessage;
use shared::net::MAX_MESSAGE_LENGTH;
use shared::net::Packable;

pub struct Network {
    socket: UdpSocket,
}

impl Network {
    pub fn new(addr: SocketAddr) -> io::Result<Network> {
        // let the os decide over port
        let local_addr = match addr {
            SocketAddr::V4(_) => "0.0.0.0:0",
            SocketAddr::V6(_) => "[::]:0",
        };
        UdpSocket::bind(local_addr).and_then(|socket| {
            if let Err(e) = socket.connect(addr) {
                return Err(e);
            }
            let network = Network {
                socket,
            };
            network.send(ClientMessage::ConnectionRequest);
            Ok(network)
        })
    }

    pub fn send(&self, msg: ClientMessage) {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        let amount = msg.pack(&mut buf).unwrap();
        self.socket.send(&buf[..amount]).unwrap();
    }

    pub fn recv_until(&self, until: Instant) -> io::Result<Option<ServerMessage>> {
        // first make sure we read a message if there are any
        self.socket.set_nonblocking(true).unwrap();
        let result = self.recv();
        self.socket.set_nonblocking(false).unwrap();
        let mut option = result?;

        // if there was no message, wait for one until time out
        if option.is_none() {
            let now = Instant::now();
            if until <= now {
                return Ok(None);
            }
            self.socket.set_read_timeout(Some(until - now)).unwrap();
            option = self.recv()?;
        }
        Ok(option)
    }

    // reads messages until there is a valid one or an error occurs
    // time out errors are transformed into None
    fn recv(&self) -> io::Result<Option<ServerMessage>> {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        loop {
            match self.socket.recv(&mut buf) {
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