use std::io::Cursor;

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
pub enum ServerMessage {
    ConnectionConfirm(u64),
    EchoResponse(u64),
    Kick,
    //SnapShot(Model),
}