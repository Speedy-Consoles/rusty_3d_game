use std::time::Instant;
use std::io;
use std::net::ToSocketAddrs;
use std::net::UdpSocket;
use std::io::ErrorKind;

use shared::model::Model;
use shared::model::world::character::CharacterInput;
use shared::net::ServerMessage;
use shared::net::ClientMessage;
use shared::net::MAX_MESSAGE_LENGTH;

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
            Ok(socket) => {
                socket.set_nonblocking(true).unwrap();
                Ok(RemoteServerInterface {
                    socket,
                    connection_state: Disconnected,
                })
            },
            Err(e) => Err(e),
        }
    }

    pub fn connect<A: ToSocketAddrs>(&mut self, addr: A) -> io::Result<()> {
        self.socket.connect(addr)?;
        self.send(ClientMessage::ConnectionRequest);
        Ok(())
    }

    fn send(&mut self, msg: ClientMessage) {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        msg.pack(&mut buf).unwrap();
        self.socket.send(&buf).unwrap();
    }

    fn recv(&self) -> Option<ServerMessage> {
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

impl ServerInterface for RemoteServerInterface {
    fn tick(&mut self, model: &mut Model, input: CharacterInput) {
        match self.connection_state {
            Connected => (),
            _ => return,
        }
        self.socket.set_nonblocking(true).unwrap();
        self.send(ClientMessage::EchoRequest(42));
        self.socket.set_nonblocking(true).unwrap();
        while let Some(msg) = self.recv() {
            println!("{:?}", msg);
        }
        // TODO
        self.socket.set_nonblocking(false).unwrap();
    }

    fn handle_traffic(&mut self, until: Instant) {
        loop {
            let now = Instant::now();
            if until <= now {
                break;
            }
            self.socket.set_read_timeout(Some(until - now)).unwrap();
            if let Some(msg) = self.recv() {
                // TODO
            }
        }
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