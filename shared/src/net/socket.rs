use std::io;
use std::io::ErrorKind;
use std::time::Instant;
use std::time::Duration;
use std::marker::PhantomData;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::hash::Hash;
use std::iter;

use arrayvec::ArrayVec;

use net::MAX_MESSAGE_LENGTH;
use net::Packable;
use net::Message;
use consts;
use consts::MAX_UNACKED_MESSAGES;

struct SentMessage {
    id: u64,
    data: ArrayVec<[u8; MAX_MESSAGE_LENGTH]>,
}

pub struct ConnectionData {
    sent_messages: VecDeque<SentMessage>, // TODO use byte buffer instead
    next_msg_id: u64,
    ack: u64,
    resend: bool,
    send_message_dropped: bool,
    last_ack: Instant,
}

impl ConnectionData {
    pub fn new() -> ConnectionData {
        ConnectionData {
            sent_messages: VecDeque::new(),
            next_msg_id: 0,
            ack: 0,
            resend: false,
            send_message_dropped: false,
            last_ack: Instant::now(),
        }
    }

    pub fn timed_out(&self) -> bool {
        self.send_message_dropped || Instant::now() - self.last_ack > consts::ack_timeout()
    }
}

pub trait ConnectionDataWrapper {
    fn con_data(&self) -> &ConnectionData;
    fn con_data_mut(&mut self) -> &mut ConnectionData;
}

impl ConnectionDataWrapper for ConnectionData {
    fn con_data(&self) -> &ConnectionData {
        self
    }
    fn con_data_mut(&mut self) -> &mut ConnectionData {
        self
    }
}

pub trait ConnectionDataProvider<AddrType> {
    fn con_data_mut(&mut self, addr: AddrType) -> Option<&mut ConnectionData>;
}

impl<T: ConnectionDataWrapper> ConnectionDataProvider<()> for T {
    fn con_data_mut(&mut self, _addr: ()) -> Option<&mut ConnectionData> {
        Some(self.con_data_mut())
    }
}

impl<AddrType, T> ConnectionDataProvider<AddrType> for HashMap<AddrType, T>
    where
        AddrType: Eq + Hash,
        T: ConnectionDataWrapper
{
    fn con_data_mut(&mut self, addr: AddrType) -> Option<&mut ConnectionData> {
        self.get_mut(&addr).map(|wrapper| wrapper.con_data_mut())
    }
}

pub trait WrappedUdpSocket<AddrType>: Sized {
    fn send_to(&self, buf: &[u8], addr: AddrType) -> io::Result<usize>;
    fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, AddrType)>;
    fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()>;
    fn set_read_timeout(&self, Option<Duration>) -> io::Result<()>;
}

#[derive(Debug, Serialize, Deserialize)]
enum MessageHeader {
    Conless,
    Conful {
        ack: u64,
        resend: bool,
        conful_header: ConfulHeader,
    },
}

#[derive(Debug, Serialize, Deserialize)]
enum ConfulHeader {
    Reliable(u64),
    Unreliable,
    Ack,
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
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        let header = MessageHeader::Conless;
        let header_size = header.pack(&mut buf).unwrap();
        let payload_size = msg.pack(&mut buf[header_size..]).unwrap();
        let msg_size = header_size + payload_size;
        self.wrapped_udp_socket.send_to(&buf[..msg_size], addr)?;
        Ok(())
    }

    pub fn send_to_reliable(&self, msg: SendType::Reliable, addr: AddrType,
                            con_data: &mut ConnectionData) -> io::Result<()> {
        if con_data.sent_messages.len() >= MAX_UNACKED_MESSAGES {
            con_data.send_message_dropped = true;
            println!("DEBUG: Maximum number of unacked messages reached!");
            return Ok(());
        }

        let id = con_data.next_msg_id;
        con_data.next_msg_id += 1;

        let mut buf = [0; MAX_MESSAGE_LENGTH];
        let header = MessageHeader::Conful {
            ack: con_data.ack,
            resend: con_data.resend,
            conful_header: ConfulHeader::Reliable(id),
        };
        con_data.resend = false;
        let header_size = header.pack(&mut buf).unwrap();
        let payload_size = msg.pack(&mut buf[header_size..]).unwrap();
        let msg_size = header_size + payload_size;
        self.wrapped_udp_socket.send_to(&buf[..msg_size], addr)?;

        let mut data: ArrayVec<[u8; MAX_MESSAGE_LENGTH]> = iter::repeat(0).collect();
        data.truncate(payload_size);
        data.copy_from_slice(&buf[header_size..msg_size]);
        con_data.sent_messages.push_back(SentMessage { id, data });

        Ok(())
    }

    pub fn send_to_unreliable(&self, msg: SendType::Unreliable,
                              addr: AddrType, con_data: &mut ConnectionData) -> io::Result<()> {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        let header = MessageHeader::Conful {
            ack: con_data.ack,
            resend: con_data.resend,
            conful_header: ConfulHeader::Unreliable,
        };
        con_data.resend = false;
        let header_size = header.pack(&mut buf).unwrap();
        let payload_size = msg.pack(&mut buf[header_size..]).unwrap();
        let msg_size = header_size + payload_size;
        self.wrapped_udp_socket.send_to(&buf[..msg_size], addr)?;

        Ok(())
    }

    pub fn broadcast_reliable<'a, I, T>(&self, msg: SendType::Reliable,
                                        host_infos: I) -> io::Result<()>
    where
        I: Iterator<Item = (&'a AddrType, &'a mut T)>,
        T: ConnectionDataWrapper + 'a,
    {
        for (addr, wrapper) in host_infos {
            self.send_to_reliable(msg.clone(), *addr, wrapper.con_data_mut())?;
        }
        Ok(())
    }

    pub fn broadcast_unreliable<'a, I, T>(&self, msg: SendType::Unreliable,
                                       host_infos: I) -> io::Result<()>
    where
        I: Iterator<Item = (&'a AddrType, &'a mut T)>,
        T: ConnectionDataWrapper + 'a,
    {
        for (addr, wrapper) in host_infos {
            self.send_to_unreliable(msg.clone(), *addr, wrapper.con_data_mut())?;
        }
        Ok(())
    }

    pub fn recv_from_until(
        &self,
        until: Instant,
        con_data_provider: &mut ConnectionDataProvider<AddrType>,
        disc_con_data_provider: &mut ConnectionDataProvider<AddrType>, // TODO allow removing of data
    ) -> io::Result<Option<(RecvType, AddrType)>> {
        // first make sure we read a message if there are any
        self.wrapped_udp_socket.set_nonblocking(true).unwrap();
        let result = self.recv_from(None, con_data_provider, disc_con_data_provider);
        self.wrapped_udp_socket.set_nonblocking(false).unwrap();
        let mut option = result?;

        // if there was no message, wait for one until time out
        if option.is_none() {
            option = self.recv_from(Some(until), con_data_provider, disc_con_data_provider)?;
        }
        Ok(option)
    }

    // reads messages until there is a valid one or an error occurs
    // time out errors are transformed into None
    fn recv_from(
        &self,
        until: Option<Instant>,
        con_data_provider: &mut ConnectionDataProvider<AddrType>,
        disc_con_data_provider: &mut ConnectionDataProvider<AddrType>, // TODO allow removing of data
    ) -> io::Result<Option<(RecvType, AddrType)>> {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        loop {
            if let Some(until) = until {
                let now = Instant::now();
                if until <= now {
                    return Ok(None);
                }
                self.wrapped_udp_socket.set_read_timeout(Some(until - now)).unwrap();
            }
            match self.wrapped_udp_socket.recv_from(&mut buf) {
                Ok((amount, addr)) => {
                    match MessageHeader::unpack(&buf[..amount]) {
                        Ok(header) => {
                            let header_size = header.packed_size().unwrap() as usize; // TODO isn't this constant?
                            let payload_slice = &buf[header_size..amount];
                            match header {
                                MessageHeader::Conless => {
                                    match RecvType::Conless::unpack(payload_slice) {
                                        Ok(msg) => return Ok(Some((msg.into(), addr))),
                                        Err(e) => println!(
                                            "DEBUG: Received malformed message.\
                                                     Unpack error: {:?}",
                                            e,
                                        ),
                                    }
                                },
                                MessageHeader::Conful {
                                    ack: their_ack,
                                    resend: their_resend,
                                    conful_header
                                } => {
                                    if let Some(con_data) = con_data_provider.con_data_mut(addr) {
                                        self.on_ack(addr, con_data, their_ack, their_resend)?;
                                        match conful_header {
                                            ConfulHeader::Reliable(id) => {
                                                if id == con_data.ack {
                                                    match RecvType::Reliable
                                                            ::unpack(payload_slice) {
                                                        Ok(msg) => {
                                                            con_data.ack += 1;
                                                            println!(
                                                                "DEBUG: Received reliable message!"
                                                            );
                                                            return Ok(Some((msg.into(), addr)));
                                                        }
                                                        Err(e) => println!(
                                                            "DEBUG: Received malformed message.\
                                                             Unpack error: {:?}",
                                                            e,
                                                        ),
                                                    }
                                                } else if id > con_data.ack {
                                                    con_data.resend = true;
                                                    println!("DEBUG: Received early packet!");
                                                } else {
                                                    println!("DEBUG: Received late packet!");
                                                }
                                            },
                                            ConfulHeader::Unreliable => {
                                                match RecvType::Unreliable::unpack(payload_slice) {
                                                    Ok(msg) => return Ok(Some((msg.into(), addr))),
                                                    Err(e) => println!(
                                                        "DEBUG: Received malformed message. \
                                                         Unpack error: {:?}",
                                                        e,
                                                    ),
                                                }
                                            },
                                            ConfulHeader::Ack => (),
                                        }
                                    } else if let Some(con_data)
                                            = disc_con_data_provider.con_data_mut(addr) {
                                        self.on_ack(addr, con_data, their_ack, their_resend)?;
                                        match conful_header {
                                            ConfulHeader::Ack => (),
                                            _ => println!("DEBUG: Received connectionful message \
                                                           from disconnecting host!"),
                                        }
                                    } else {
                                        println!("DEBUG: Received connectionful message \
                                                  from unknown host!");
                                    }
                                },
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

    fn on_ack(&self, addr: AddrType, con_data: &mut ConnectionData,
              ack: u64, resend: bool) -> io::Result<()>{
        con_data.last_ack = Instant::now();

        // first, remove all acked messages
        loop {
            if let Some(sent_msg) = con_data.sent_messages.front() {
                if sent_msg.id >= ack {
                    break;
                }
            } else {
                break;
            }
            con_data.sent_messages.pop_front().unwrap();
        }

        // then resend all not acked messages, if requested
        if resend {
            // TODO can this get inperformant?
            for sent_message in con_data.sent_messages.iter() {
                let mut buf = [0; MAX_MESSAGE_LENGTH];
                let header = MessageHeader::Conful {
                    ack: con_data.ack,
                    resend: con_data.resend,
                    conful_header: ConfulHeader::Reliable(sent_message.id),
                };
                con_data.resend = false;
                let header_size = header.pack(&mut buf).unwrap();
                buf[header_size..].copy_from_slice(&sent_message.data);
                let msg_size = header_size + sent_message.data.len();
                self.wrapped_udp_socket.send_to(&buf[..msg_size], addr)?;
            }
        }

        Ok(())
    }
}