use std::time::Instant;
use std::io;
use std::net::ToSocketAddrs;
use std::net::UdpSocket;

use shared::model::Model;
use shared::model::world::character::CharacterInput;

use super::ConnectionState;
use super::ConnectionState::*;
use super::ServerInterface;


pub struct RemoteServerInterface {
    socket: UdpSocket,
    connection_state: ConnectionState,
}

impl RemoteServerInterface {
    pub fn new() -> io::Result<RemoteServerInterface> {
        match UdpSocket::bind("0.0.0.0:0") {
            Ok(socket) => Ok(RemoteServerInterface {
                socket,
                connection_state: Disconnected,
            }),
            Err(e) => Err(e),
        }
    }

    pub fn connect<A: ToSocketAddrs>(&mut self, addr: A) -> io::Result<()> {
        self.socket.connect(addr)
    }
}

impl ServerInterface for RemoteServerInterface {
    fn tick(&mut self, model: &mut Model, input: CharacterInput) {
        self.socket.set_nonblocking(false).unwrap();
        self.socket.send(&[1, 2, 3, 4]).unwrap();
        self.socket.set_nonblocking(true).unwrap();
        let mut buf = [0; 10];
        while let Ok(amount) = self.socket.recv(&mut buf) {
            let buf = &buf[..amount];
            println!("{:?}", buf);
        }
        // TODO
    }

    fn get_tick(&self) -> u64 {
        // TODO
        0
    }

    fn get_predicted_tick(&self) -> u64 {
        // TODO
        0
    }

    fn get_intra_tick(&self) -> f64 {
        // TODO
        0.0
    }

    fn get_next_tick_time(&self) -> Instant {
        // TODO
        Instant::now()
    }

    fn get_my_id(&self) -> Option<u64> {
        // TODO
        None
    }

    fn get_character_input(&self, tick: u64) -> Option<CharacterInput> {
        // TODO
        None
    }

    fn get_connection_state(&self) -> ConnectionState {
        self.connection_state
    }
}