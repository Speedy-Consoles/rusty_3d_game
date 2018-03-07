use std::io::Cursor;
use std::cmp::Ordering;

use bincode;
use bincode::serialize_into;
use bincode::deserialize;

use serde::Serialize;
use serde::Deserialize;

use model::Model;

pub const MAX_MESSAGE_LENGTH: usize = 1024;

pub trait Packable<'a>: Sized {
    fn unpack(buf: &'a [u8]) -> bincode::Result<Self>;
    fn pack(&self, buf: &mut [u8]) -> bincode::Result<usize>;
}

impl<'a, T: Serialize + Deserialize<'a>> Packable<'a> for T {
    fn unpack(buf: &'a [u8]) -> bincode::Result<Self> {
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
    ConnectionRequest,
    EchoRequest(u64),
    Leave,
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

    pub fn get_tick(&self) -> u64 {
        self.tick
    }

    pub fn get_model(&self) -> &Model {
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
    ConnectionConfirm(u64),
    EchoResponse(u64),
    Kick,
    Snapshot(Snapshot),
}