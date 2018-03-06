use std::time::Instant;
use std::time::Duration;
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
    my_id: Option<u64>,
}

impl RemoteServerInterface {
    pub fn new() -> io::Result<RemoteServerInterface> {
        // let the os decide over port
        UdpSocket::bind("0.0.0.0:0").map(|socket| {
            socket.set_nonblocking(false).unwrap();
            RemoteServerInterface {
                socket,
                connection_state: Disconnected,
                my_id: None,
            }
        })
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
        match self.connection_state { // TODO
            Connected => (),
            _ => return,
        }
        self.socket.set_nonblocking(true).unwrap();
        // TODO
        self.send(ClientMessage::EchoRequest(42));
        self.socket.set_nonblocking(false).unwrap();
    }

    fn handle_traffic(&mut self, until: Instant) {
        use shared::net::ServerMessage::*;
        loop {
            let now = Instant::now();
            if until <= now {
                break;
            }
            self.socket.set_read_timeout(Some(until - now)).unwrap();
            if let Some(msg) = self.recv() {
                println!("{:?}", msg);
                match msg {
                    ConnectionConfirm(id) => self.my_id = Some(id),
                    _ => (), // TODO
                }
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
        Instant::now() + Duration::from_secs(1)
    }

    fn get_my_id(&self) -> Option<u64> {
        self.my_id
    }

    fn get_character_input(&self, tick: u64) -> Option<CharacterInput> {
        // TODO
        None
    }

    fn get_connection_state(&self) -> ConnectionState {
        self.connection_state
    }
}