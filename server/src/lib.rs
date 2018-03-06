extern crate shared;

use std::net::UdpSocket;

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
        // Receives a single datagram message on the socket. If `buf` is too small to hold
        // the message, it will be cut off.
        let mut buf = [0; 10];
        while let Ok((amt, src)) = self.socket.recv_from(&mut buf) {
            // Redeclare `buf` as slice of the received data and send reverse data back to origin.
            let buf = &mut buf[..amt];
            buf.reverse();
            self.socket.send_to(buf, &src).unwrap();
        }
    } // the socket is closed here
}