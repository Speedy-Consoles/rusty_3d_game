use std::io;
use std::io::ErrorKind;
use std::time::Instant;
use std::time::Duration;

use net::MAX_MESSAGE_LENGTH;
use net::Packable;

pub trait Socket<SendType: Packable, RecvType: Packable, AddrType, CheckedRecvType> {
    fn send_impl(&self, buf: &[u8], addr: AddrType) -> io::Result<usize>;
    fn recv_impl(&self, buf: &mut [u8]) -> io::Result<(usize, AddrType)>;
    fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()>;
    fn set_read_timeout(&self, Option<Duration>) -> io::Result<()>;
    fn check_msg(&self, msg: RecvType, addr: AddrType) -> Option<CheckedRecvType>;

    fn send_to(&self, msg: &SendType, addr: AddrType) {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        let amount = msg.pack(&mut buf).unwrap();
        self.send_impl(&buf[..amount], addr).unwrap();
    }

    fn recv_from_until(&self, until: Instant) -> io::Result<Option<CheckedRecvType>> {
        // first make sure we read a message if there are any
        self.set_nonblocking(true).unwrap();
        let result = self.recv_from();
        self.set_nonblocking(false).unwrap();
        let mut option = result?;

        // if there was no message, wait for one until time out
        if option.is_none() {
            let now = Instant::now();
            if until <= now {
                return Ok(None);
            }
            self.set_read_timeout(Some(until - now)).unwrap();
            option = self.recv_from()?;
        }
        Ok(option)
    }

    // reads messages until there is a valid one or an error occurs
    // time out errors are transformed into None
    fn recv_from(&self) -> io::Result<Option<CheckedRecvType>> {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        loop {
            match self.recv_impl(&mut buf) {
                Ok((amount, addr)) => {
                    match RecvType::unpack(&buf[..amount]) {
                        Ok(msg) => {
                            if let Some(checked_msg) = self.check_msg(msg, addr) {
                                return Ok(Some(checked_msg));
                            }
                        },
                        Err(e) => println!(
                            "DEBUG: Received malformed message. Unpack error: {:?}",
                            e,
                        ),
                    }
                },
                Err(e) => {
                    match e.kind() {
                        ErrorKind::WouldBlock | ErrorKind::TimedOut => return Ok(None),
                        _ => return Err(e),
                    };
                }
            }
        }
    }
}