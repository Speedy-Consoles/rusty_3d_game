extern crate shared;

use std::net::UdpSocket;

use shared::net::MAX_MESSAGE_LENGTH;
use shared::net::ClientMessage;
use shared::net::ServerMessage;

pub struct Server {
    socket: UdpSocket,
}

impl Server {
    pub fn new() -> Server {
        Server {
            socket: UdpSocket::bind("127.0.0.1:51946").unwrap(),
        }
    }

    pub fn run(&mut self) {
        let mut recv_buf = [0; MAX_MESSAGE_LENGTH];
        let mut send_buf = [0; MAX_MESSAGE_LENGTH];
        while let Ok((amt, src)) = self.socket.recv_from(&mut recv_buf) {
            match ClientMessage::unpack(&recv_buf[..amt]) {
                Ok(ClientMessage::ConnectionRequest) => {
                    // TODO
                },
                Ok(ClientMessage::EchoRequest(id)) => {
                    ServerMessage::EchoResponse(id).pack(&mut send_buf).unwrap();
                    self.socket.send_to(&send_buf, &src).unwrap(); // TODO get rid of unwrap
                },
                Ok(ClientMessage::Leave) => {
                    // TODO
                },
                Err(e) => println!("Received invalid message: {}", e),
            }
        }
    }
}