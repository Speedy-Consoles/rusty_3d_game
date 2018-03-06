use bincode;
use bincode::serialize_into;
use bincode::deserialize;

pub const MAX_MESSAGE_LENGTH: usize = 1024;

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientMessage {
    ConnectionRequest,
    EchoRequest(u64),
    Leave,
}

impl ClientMessage {
    pub fn unpack(buf: &[u8]) -> bincode::Result<Self> {
        deserialize(buf)
    }

    pub fn pack(&self, buf: &mut [u8]) -> bincode::Result<()> {
        serialize_into(buf, self)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerMessage {
    ConnectionConfirm(u64),
    EchoResponse(u64),
}

impl ServerMessage {
    pub fn unpack(buf: &[u8]) -> bincode::Result<Self> {
        deserialize(buf)
    }

    pub fn pack(&self, buf: &mut [u8]) -> bincode::Result<()> {
        serialize_into(buf, self)
    }
}