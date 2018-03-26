pub mod socket;

use std::io::Cursor;
use std::cmp::Ordering;

use bincode;

use serde::Serialize;
use serde::de::DeserializeOwned;

use tick_time::TickInstant;
use model::Model;
use model::world::character::CharacterInput;

pub const MAX_MESSAGE_LENGTH: usize = 1024;

pub trait Packable: Sized {
    fn unpack(buf: &[u8]) -> bincode::Result<Self>;
    fn pack(&self, buf: &mut [u8]) -> bincode::Result<usize>;
    fn packed_size(&self) -> bincode::Result<u64>;
}

impl<T: Serialize + DeserializeOwned> Packable for T {
    fn unpack(buf: &[u8]) -> bincode::Result<Self> {
        bincode::deserialize(buf)
    }

    fn pack(&self, buf: &mut [u8]) -> bincode::Result<usize> {
        let mut cursor = Cursor::new(buf);
        bincode::serialize_into(&mut cursor, self)?;
        Ok(cursor.position() as usize)
    }

    fn packed_size(&self) -> bincode::Result<u64> {
        bincode::serialized_size(self)
    }
}

pub trait Message: Sized {
    type Conless: Serialize + DeserializeOwned + Into<Self>;
    type Reliable: Serialize + DeserializeOwned + Into<Self> + Clone; // TODO maybe remove clone?
    type Unreliable: Serialize + DeserializeOwned + Into<Self> + Clone;
}

pub enum ClientMessage {
    Conless(ConlessClientMessage),
    Reliable(ReliableClientMessage),
    Unreliable(UnreliableClientMessage),
}

impl Message for ClientMessage {
    type Conless = ConlessClientMessage;
    type Reliable = ReliableClientMessage;
    type Unreliable = UnreliableClientMessage;
}

impl From<ConlessClientMessage> for ClientMessage {
    fn from(msg: ConlessClientMessage) -> Self {
        ClientMessage::Conless(msg)
    }
}

impl From<ReliableClientMessage> for ClientMessage {
    fn from(msg: ReliableClientMessage) -> Self {
        ClientMessage::Reliable(msg)
    }
}

impl From<UnreliableClientMessage> for ClientMessage {
    fn from(msg: UnreliableClientMessage) -> Self {
        ClientMessage::Unreliable(msg)
    }
}

pub enum ServerMessage {
    Conless(ConlessServerMessage),
    Reliable(ReliableServerMessage),
    Unreliable(UnreliableServerMessage),
}

impl Message for ServerMessage {
    type Conless = ConlessServerMessage;
    type Reliable = ReliableServerMessage;
    type Unreliable = UnreliableServerMessage;
}

impl From<ConlessServerMessage> for ServerMessage {
    fn from(msg: ConlessServerMessage) -> Self {
        ServerMessage::Conless(msg)
    }
}

impl From<ReliableServerMessage> for ServerMessage {
    fn from(msg: ReliableServerMessage) -> Self {
        ServerMessage::Reliable(msg)
    }
}

impl From<UnreliableServerMessage> for ServerMessage {
    fn from(msg: UnreliableServerMessage) -> Self {
        ServerMessage::Unreliable(msg)
    }
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
pub enum ConlessClientMessage {
    ConnectionRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReliableClientMessage {
    DisconnectRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnreliableClientMessage {
    InputMessage { tick: u64, input: CharacterInput, },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ConlessServerMessage {
    ConnectionConfirm(u64),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReliableServerMessage {
    ConnectionClose,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UnreliableServerMessage {
    SnapshotMessage(Snapshot),
    InputAck {
        input_tick: u64,
        arrival_tick_instant: TickInstant,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    tick: u64,
    model: Model
}