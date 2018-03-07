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
use shared::net::Packable;
use shared::net::Snapshot;
use shared::net::MAX_MESSAGE_LENGTH;

use super::ConnectionState;
use super::ConnectionState::*;
use super::ServerInterface;
use super::TickInfo;


pub struct RemoteServerInterface {
    socket: UdpSocket,
    connection_state: ConnectionState,
    tick_info: Option<TickInfo>,
    tick_lag: Option<u64>,
    my_player_id: Option<u64>,
    last_snapshot: Option<Snapshot>,
}

impl RemoteServerInterface {
    pub fn new<A: ToSocketAddrs>(addr: A) -> io::Result<RemoteServerInterface> {
        // let the os decide over port
        UdpSocket::bind("0.0.0.0:0").and_then(|socket| {
            if let Err(e) = socket.connect(addr) {
                return Err(e);
            }
            let mut rsi = RemoteServerInterface {
                socket,
                connection_state: Disconnected,
                tick_info: None,
                tick_lag: None,
                my_player_id: None,
                last_snapshot: None,
            };
            rsi.send(ClientMessage::ConnectionRequest);
            Ok(rsi)
        })
    }

    fn send(&mut self, msg: ClientMessage) {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        let amount = msg.pack(&mut buf).unwrap();
        self.socket.send(&buf[..amount]).unwrap();
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
        if let Some(ref snapshot) = self.last_snapshot {
            *model = snapshot.get_model().clone();
        }
        // TODO send input
        self.send(ClientMessage::EchoRequest(42));
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
                let now = Instant::now();
                println!("{:?}", msg);
                match msg {
                    ConnectionConfirm(id) => {
                        self.my_player_id = Some(id);
                        self.connection_state = Connected;
                    },
                    Snapshot(new_snapshot) => {
                        let replace = if let Some(ref last_snapshot) = self.last_snapshot {
                            *last_snapshot < new_snapshot
                        } else {
                            true
                        };
                        if replace {
                            self.tick_info = Some(TickInfo {
                                tick: new_snapshot.get_tick(),
                                tick_time: now,
                            });
                            self.last_snapshot = Some(new_snapshot)
                        }
                    },
                    _ => (), // TODO
                }
            }
        }
    }

    fn get_tick_info(&self) -> Option<TickInfo> {
        self.tick_info
    }

    fn get_tick_lag(&self) -> Option<u64> {
        self.tick_lag
    }

    fn get_my_player_id(&self) -> Option<u64> {
        self.my_player_id
    }

    fn get_character_input(&self, tick: u64) -> Option<CharacterInput> {
        // TODO
        None
    }

    fn get_connection_state(&self) -> ConnectionState {
        self.connection_state
    }
}