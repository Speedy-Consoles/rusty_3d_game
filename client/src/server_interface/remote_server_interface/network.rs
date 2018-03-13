use std::io;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::net::UdpSocket;
use std::time::Duration;

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

    pub fn recv(&self, read_time_out: Option<Duration>) -> Option<ServerMessage> {
        self.socket.set_read_timeout(read_time_out).unwrap();
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        match self.socket.recv(&mut buf) {
            Ok(amount) => {
                match ServerMessage::unpack(&buf[..amount]) {
                    Ok(msg) => Some(msg),
                    Err(e) => {
                        println!("{:?}", e);
                        None
                    },
                }
            },
            Err(e) => {
                match e.kind() {
                    ErrorKind::WouldBlock | ErrorKind::TimedOut => (),
                    _ => println!("{:?}", e),
                };
                None
            }
        }
    }
}