use std::io;
use std::io::ErrorKind;
use std::time::Instant;
use std::time::Duration;
use std::marker::PhantomData;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::hash::Hash;

use net::MAX_MESSAGE_LENGTH;
use net::Packable;
use net::Message;

pub trait RecvQueueWrapper<RecvType> {
    fn recv_queue(&mut self) -> Option<&mut RecvQueue<RecvType>>;
}

pub struct RecvQueue<RecvType> {
    messages: HashMap<u64, RecvType>,
    oldest_id: u64,
}

impl<RecvType> RecvQueue<RecvType> {
    pub fn new() -> Self {
        RecvQueue {
            messages: HashMap::new(),
            oldest_id: 0,
        }
    }
}

pub struct SendQueue<SendType> {
    messages: VecDeque<(u64, SendType)>,
}

impl<SendType> SendQueue<SendType> {
    pub fn new() -> Self {
        SendQueue {
            messages: VecDeque::new(),
        }
    }
}

pub trait RecvQueueProvider<AddrType, RecvType> {
    fn recv_queue(&mut self, addr: AddrType) -> Option<&mut RecvQueue<RecvType>>;
}

impl<AddrType, RecvType, T> RecvQueueProvider<AddrType, RecvType> for HashMap<AddrType, T>
where
    AddrType: Eq + Hash,
    T: RecvQueueWrapper<RecvType>
{
    fn recv_queue(&mut self, addr: AddrType) -> Option<&mut RecvQueue<RecvType>> {
        self.get_mut(&addr).and_then(|wrapper| wrapper.recv_queue())
    }
}

pub trait WrappedUdpSocket<AddrType>: Sized {
    fn send_to(&self, buf: &[u8], addr: AddrType) -> io::Result<usize>;
    fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, AddrType)>;
    fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()>;
    fn set_read_timeout(&self, Option<Duration>) -> io::Result<()>;
}

#[derive(Debug, Serialize, Deserialize)]
enum SocketMessage<T: Message> {
    Conless(T::Conless),
    Reliable(T::Reliable, u64),
    Unreliable(T::Unreliable),
}

pub struct ReliableSocket<
    SendType: Message,
    RecvType: Message,
    AddrType: 'static + Copy,
    WrappedUdpSocketType: WrappedUdpSocket<AddrType>
> {
    phantom_send: PhantomData<SendType>,
    phantom_recv: PhantomData<RecvType>,
    phantom_addr: PhantomData<AddrType>,
    wrapped_udp_socket: WrappedUdpSocketType,
}

impl<
    SendType: Message,
    RecvType: Message,
    AddrType: 'static + Copy,
    WrappedUdpSocketType: WrappedUdpSocket<AddrType>
> ReliableSocket<SendType, RecvType, AddrType, WrappedUdpSocketType> {
    pub fn new(wrapped_udp_socket: WrappedUdpSocketType) -> Self {
        ReliableSocket {
            phantom_send: PhantomData,
            phantom_recv: PhantomData,
            phantom_addr: PhantomData,
            wrapped_udp_socket,
        }
    }

    pub fn send_to_conless(&self, msg: SendType::Conless, addr: AddrType) -> io::Result<()> {
        self.send_to(SocketMessage::Conless(msg), addr)
    }

    pub fn send_to_reliable(&self, msg: SendType::Reliable, addr: AddrType,
                            send_queue: &mut SendQueue<SendType>) -> io::Result<()> {
        let msg_id = 0; // TODO
        self.send_to(SocketMessage::Reliable(msg, msg_id), addr)
    }

    pub fn send_to_unreliable(&self, msg: SendType::Unreliable, addr: AddrType) -> io::Result<()> {
        self.send_to(SocketMessage::Unreliable(msg), addr)
    }

    pub fn broadcast_reliable<'a, I>(
        &self,
        msg: SendType::Reliable,
        send_queues: I
    ) -> io::Result<()>
    where
        I: Iterator<Item = (&'a AddrType, &'a mut SendQueue<SendType>)>,
        SendType: 'a,
    {
        for (addr, send_queue) in send_queues {
            self.send_to_reliable(msg.clone(), *addr, send_queue)?;
        }
        Ok(())
    }

    pub fn broadcast_unreliable<'a, I>(&self, msg: SendType::Unreliable, addrs: I) -> io::Result<()>
    where I: Iterator<Item = &'a AddrType> {
        for addr in addrs {
            self.send_to_unreliable(msg.clone(), *addr)?;
        }
        Ok(())
    }

    fn send_to(&self, msg: SocketMessage<SendType>, addr: AddrType) -> io::Result<()> {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        let amount = msg.pack(&mut buf).unwrap();
        self.wrapped_udp_socket.send_to(&buf[..amount], addr)?;
        Ok(())
    }

    pub fn recv_from_until(
        &self,
        until: Instant,
        recv_queue_provider: &mut RecvQueueProvider<AddrType, RecvType>
    ) -> io::Result<Option<(RecvType, AddrType)>> {
        // first make sure we read a message if there are any
        self.wrapped_udp_socket.set_nonblocking(true).unwrap();
        let result = self.recv_from(recv_queue_provider);
        self.wrapped_udp_socket.set_nonblocking(false).unwrap();
        let mut option = result?;

        // if there was no message, wait for one until time out
        if option.is_none() {
            let now = Instant::now();
            if until <= now {
                return Ok(None);
            }
            self.wrapped_udp_socket.set_read_timeout(Some(until - now)).unwrap();
            option = self.recv_from(recv_queue_provider)?;
        }
        Ok(option)
    }

    // reads messages until there is a valid one or an error occurs
    // time out errors are transformed into None
    fn recv_from(&self, recv_queue_provider: &mut RecvQueueProvider<AddrType, RecvType>)
    -> io::Result<Option<(RecvType, AddrType)>> {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        loop {
            match self.wrapped_udp_socket.recv_from(&mut buf) {
                Ok((amount, addr)) => {
                    match SocketMessage::<RecvType>::unpack(&buf[..amount]) {
                        Ok(socket_msg) => {
                            if let SocketMessage::Conless(msg) = socket_msg {
                                return Ok(Some((msg.into(), addr)));
                            } else if let Some(queue) = recv_queue_provider.recv_queue(addr) {
                                return Ok(Some((match socket_msg {
                                    SocketMessage::Reliable(msg, _msg_id) => {
                                        // TODO
                                        msg.into()
                                    },
                                    SocketMessage::Unreliable(msg) => msg.into(),
                                    _ => unreachable!(),
                                }, addr)));
                            }
                            println!("DEBUG: Received connectionful message from unknown host!");
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