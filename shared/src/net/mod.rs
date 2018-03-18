pub mod socket;

use std::io::Cursor;
use std::cmp::Ordering;

use bincode;
use bincode::serialize_into;
use bincode::deserialize;

use serde::Serialize;
use serde::de::DeserializeOwned;

use tick_time::TickInstant;
use model::Model;
use model::world::character::CharacterInput;


pub const MAX_MESSAGE_LENGTH: usize = 1024;

pub trait Packable: Sized {
    fn unpack(buf: &[u8]) -> bincode::Result<Self>;
    fn pack(&self, buf: &mut [u8]) -> bincode::Result<usize>;
}

impl<T: Serialize + DeserializeOwned> Packable for T {
    fn unpack(buf: &[u8]) -> bincode::Result<Self> {
        deserialize(buf)
    }

    fn pack(&self, buf: &mut [u8]) -> bincode::Result<usize> {
        let mut cursor = Cursor::new(buf);
        serialize_into(&mut cursor, self)?;
        Ok(cursor.position() as usize)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    Connected(ConClientMessage),
    Connectionless(ConLessClientMessage),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ConClientMessage {
    DisconnectRequest,
    InputMessage { tick: u64, input: CharacterInput, },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ConLessClientMessage {
    ConnectionRequest,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    tick: u64,
    model: Model
}

impl Snapshot {
    pub fn new(tick: u64, model: &Model) -> Snapshot {
        Snapshot {
            tick,
            model: model.clone(),
        }
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    pub fn model(&self) -> &Model {
        &self.model
    }
}

impl PartialEq for Snapshot {
    fn eq(&self, other: &Snapshot) -> bool {
        self.tick == other.tick
    }
}

impl PartialOrd for Snapshot {
    fn partial_cmp(&self, other: &Snapshot) -> Option<Ordering> {
        if self.tick == other.tick {
            None
        } else {
            Some(self.tick.cmp(&other.tick))
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    Connected(ConServerMessage),
    Connectionless(ConLessServerMessage),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ConLessServerMessage {
    ConnectionConfirm(u64),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ConServerMessage {
    ConnectionClose(ConnectionCloseReason),
    SnapshotMessage(Snapshot),
    InputAck {
        input_tick: u64,
        arrival_tick_instant: TickInstant,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ConnectionCloseReason {
    UserDisconnect,
    TimedOut,
    Kicked,
}