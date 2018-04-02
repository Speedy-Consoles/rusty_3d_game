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
use consts::ACK_DURATION_SIGMA_FACTOR;
use online_distribution::OnlineDistribution;

use self::SocketEvent::*;
use self::CheckedMessage::*;
use self::ConMessage::*;
use self::ConnectionEndReason::*;

pub trait WrappedUdpSocket<AddrType>: Sized {
    fn send_to(&self, buf: &[u8], addr: AddrType) -> io::Result<usize>;
    fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, AddrType)>;
    fn set_nonblocking(&self, nonblocking: bool) -> io::Result<()>;
    fn set_read_timeout(&self, Option<Duration>) -> io::Result<()>;
}

pub enum ConnectionEndReason {
    TimedOut,
    Reset,
}

pub enum SocketEvent<AddrType, RecvType: Message> {
    MessageReceived(CheckedMessage<AddrType, RecvType>),
    DoneDisconnecting(u64),
    DisconnectingConnectionEnd {
        reason: ConnectionEndReason,
        con_id: u64,
        // TODO unacked messages
    },
    ConnectionEnd {
        reason: ConnectionEndReason,
        con_id: u64,
        // TODO unacked messages
    },
    // TODO when can an io error occur? Is the network completely broken after that?
    NetworkError(io::Error),
}

pub enum CheckedMessage<AddrType, RecvType: Message> {
    Conless {
        addr: AddrType,
        con_id: Option<ConId>,
        clmsg: RecvType::Conless,
    },
    Conful {
        con_id: u64,
        cmsg: ConMessage<RecvType>,
    }
}

pub enum ConMessage<T: Message> {
    Reliable(T::Reliable),
    Unreliable(T::Unreliable),
}

pub type ConId = u64;

enum SendReliableError {
    BufferFull,
    NetworkError(io::Error)
}

impl From<io::Error> for SendReliableError {
    fn from(err: io::Error) -> SendReliableError {
        SendReliableError::NetworkError(err)
    }
}

struct SentMessage {
    id: u64,
    send_time: Instant,
    data: ArrayVec<[u8; MAX_MESSAGE_LENGTH]>,
}

struct Connection<AddrType: Copy> {
    addr: AddrType,
    sent_messages: VecDeque<SentMessage>, // TODO use byte buffer instead
    ack_distribution: OnlineDistribution<Duration>,
    next_msg_id: u64,
    my_ack: u64,
    my_resend: bool,
    their_ack: u64,
    their_resend: bool,
    last_recv_time: Instant,
    last_resend_time: Option<Instant>,
    disconnecting: bool,
}

impl<AddrType: Copy> Connection<AddrType> {
    fn on_ack(&mut self, their_ack: u64, their_resend: bool) {
        self.last_recv_time = Instant::now();
        self.their_ack = their_ack;
        self.their_resend = their_resend;

        // remove all acked messages
        loop {
            if let Some(sent_msg) = self.sent_messages.front() {
                if sent_msg.id >= their_ack {
                    break;
                }
            } else {
                break;
            }
            self.sent_messages.pop_front().unwrap();
            self.last_resend_time = None;
        }
    }

    fn send_reliable<M, S, T: Message>(&mut self, msg: M::Reliable, socket: &S)
        -> Result<(), SendReliableError>
    where
        M: Message,
        S: WrappedUdpSocket<AddrType>,
    {
        if self.sent_messages.len() >= MAX_UNACKED_MESSAGES {
            println!("DEBUG: Maximum number of unacked messages reached!");
            return Err(SendReliableError::BufferFull);
        }

        let now = Instant::now();

        let id = self.next_msg_id;
        self.next_msg_id += 1;

        let mut buf = [0; MAX_MESSAGE_LENGTH];
        let header = MessageHeader::Conful {
            ack: self.my_ack,
            resend: self.my_resend,
            conful_header: ConfulHeader::Reliable(id),
        };
        self.my_resend = false;
        let header_size = header.pack(&mut buf).unwrap();
        let payload_size = msg.pack(&mut buf[header_size..]).unwrap();
        let msg_size = header_size + payload_size;
        socket.send_to(&buf[..msg_size], self.addr)?;

        let mut data: ArrayVec<[u8; MAX_MESSAGE_LENGTH]> = iter::repeat(0).collect();
        data.truncate(payload_size);
        data.copy_from_slice(&buf[header_size..msg_size]);
        self.sent_messages.push_back(SentMessage { id, send_time: now, data });

        Ok(())
    }

    fn send_unreliable<M, S, RecvType: Message>(&mut self, msg: M::Unreliable, socket: &S,
        event_queue: &mut VecDeque<SocketEvent<AddrType, RecvType>>)
    where
        M: Message,
        S: WrappedUdpSocket<AddrType>,
    {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        let header = MessageHeader::Conful {
            ack: self.my_ack,
            resend: self.my_resend,
            conful_header: ConfulHeader::Unreliable,
        };
        self.my_resend = false;
        let header_size = header.pack(&mut buf).unwrap();
        let payload_size = msg.pack(&mut buf[header_size..]).unwrap();
        let msg_size = header_size + payload_size;
        if let Err(e) = socket.send_to(&buf[..msg_size], self.addr) {
            event_queue.push_back(SocketEvent::NetworkError(e));
        }
    }

    fn send_ack<S, RecvType: Message>(&mut self, socket: &S,
        event_queue: &mut VecDeque<SocketEvent<AddrType, RecvType>>)
    where
        S: WrappedUdpSocket<AddrType>,
    {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        let header = MessageHeader::Conful {
            ack: self.my_ack,
            resend: self.my_resend,
            conful_header: ConfulHeader::Ack,
        };
        self.my_resend = false;
        let header_size = header.pack(&mut buf).unwrap();
        if let Err(e) = socket.send_to(&buf[..header_size], self.addr) {
            event_queue.push_back(SocketEvent::NetworkError(e));
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum MessageHeader {
    Conless,
    Conful {
        ack: u64,
        resend: bool,
        conful_header: ConfulHeader,
    },
    ConReset,
}

#[derive(Debug, Serialize, Deserialize)]
enum ConfulHeader {
    Reliable(u64),
    Unreliable,
    Ack,
}

pub struct ReliableSocket<
    AddrType: 'static + Copy,
    SendType: Message,
    RecvType: Message,
    WrappedUdpSocketType: WrappedUdpSocket<AddrType>
> {
    next_connection_id: ConId,
    connections: HashMap<ConId, Connection<AddrType>>, // TODO consider making this an array
    remove_connections_buffer: Vec<ConId>,
    con_ids_by_addr: HashMap<AddrType, ConId>, // TODO consider replacing this by linear search
    socket: WrappedUdpSocketType,
    next_tick_time: Instant,
    timeout_duration: Duration,
    timeout_duration_disconnecting: Duration,
    phantom_send: PhantomData<SendType>,
    phantom_recv: PhantomData<RecvType>,
}

impl<
    AddrType: 'static + Copy + Hash + Eq,
    SendType: Message,
    RecvType: Message,
    WrappedUdpSocketType: WrappedUdpSocket<AddrType>
> ReliableSocket<AddrType, SendType, RecvType, WrappedUdpSocketType> {
    pub fn new(wrapped_udp_socket: WrappedUdpSocketType, ack_timeout: Duration,
               ack_timeout_disconnecting: Duration) -> Self {
        ReliableSocket {
            next_connection_id: 0,
            connections: HashMap::new(),
            remove_connections_buffer: Vec::new(),
            con_ids_by_addr: HashMap::new(),
            socket: wrapped_udp_socket,
            next_tick_time: Instant::now(),
            timeout_duration: ack_timeout,
            timeout_duration_disconnecting: ack_timeout_disconnecting,
            phantom_send: PhantomData,
            phantom_recv: PhantomData,
        }
    }

    pub fn connect(&mut self, addr: AddrType) -> ConId {
        if self.con_ids_by_addr.contains_key(&addr) {
            println!("ERROR: Tried to created connection with address of existing connection!")
        }
        let id = self.next_connection_id;
        self.next_connection_id += 1;
        // TODO avoid memory allocation on new connections
        self.connections.insert(id, Connection {
            addr,
            sent_messages: VecDeque::new(),
            ack_distribution: OnlineDistribution::new(consts::initial_ack_duration_guess()),
            next_msg_id: 0,
            my_ack: 0,
            my_resend: false,
            their_ack: 0,
            their_resend: false,
            last_recv_time: Instant::now(),
            last_resend_time: None,
            disconnecting: false,
        });
        self.con_ids_by_addr.insert(addr, id);
        id
    }

    pub fn disconnect(&mut self, con_id: ConId) {
        if let Some(con) = self.connections.get_mut(&con_id) {
            con.disconnecting = true;
        } else {
            println!("ERROR: Tried to disconnect non-existing connection");
        }
    }

    pub fn terminate(&mut self, con_id: ConId,
                     event_queue: &mut VecDeque<SocketEvent<AddrType, RecvType>>) {
        if let Some(mut con) = self.connections.remove(&con_id) {
            self.con_ids_by_addr.remove(&con.addr).unwrap();
            con.send_ack(&self.socket, event_queue);
        } else {
            println!("ERROR: Tried to disconnect non-existing connection");
        }
    }

    pub fn do_tick(&mut self, event_queue: &mut VecDeque<SocketEvent<AddrType, RecvType>>) {
        // TODO maybe also resend?
        let now = Instant::now();
        for (&con_id, con) in self.connections.iter_mut() {
            // check if didn't hear anything for too long
            if let Some(&SentMessage { send_time, .. }) = con.sent_messages.front() {
                let ack_silence = now - send_time;
                let timed_out = if con.disconnecting {
                    ack_silence > self.timeout_duration_disconnecting
                } else {
                    ack_silence > self.timeout_duration
                };
                if timed_out {
                    self.remove_connections_buffer.push(con_id);
                    continue;
                }
            }

            // resend if ack is low for too long or if requested
            let resend_timeout = con.ack_distribution.mean()
                    + con.ack_distribution.sigma_dev(ACK_DURATION_SIGMA_FACTOR);
            let mut resend = con.their_resend;
            if let Some(last_resend_time) = con.last_resend_time {
                resend |= now > last_resend_time + resend_timeout;
            }
            if resend {
                if con.their_resend {
                    println!("DEBUG: Resending to {} because of request!", con_id);
                }
                println!("DEBUG: Resending to {} because of resend timeout!", con_id);
                // TODO can this get inperformant?
                for sent_message in con.sent_messages.iter() {
                    let mut buf = [0; MAX_MESSAGE_LENGTH];
                    let header = MessageHeader::Conful {
                        ack: con.my_ack,
                        resend: con.my_resend,
                        conful_header: ConfulHeader::Reliable(sent_message.id),
                    };
                    let header_size = header.pack(&mut buf).unwrap();
                    buf[header_size..].copy_from_slice(&sent_message.data);
                    let msg_size = header_size + sent_message.data.len();
                    if let Err(err) = self.socket.send_to(&buf[..msg_size], con.addr) {
                        event_queue.push_back(NetworkError(err));
                    }
                }
                con.last_resend_time = Some(now);
                con.their_resend = false;
            }
        }

        for con_id in self.remove_connections_buffer.drain(..) {
            // TODO add info about unsent messages
            let con = self.connections.remove(&con_id).unwrap();
            self.con_ids_by_addr.remove(&con.addr).unwrap();
            if con.disconnecting {
                event_queue.push_back(SocketEvent::DisconnectingConnectionEnd {
                    reason: TimedOut,
                    con_id,
                });
            } else {
                event_queue.push_back(SocketEvent::ConnectionEnd {
                    reason: TimedOut,
                    con_id,
                });
            }
        }

        // TODO make flexible
        self.next_tick_time = now + Duration::new(0, 8333333);
    }

    pub fn next_tick_time(&self) -> Option<Instant> {
        // TODO make flexible
        if self.connections.is_empty() {
            None
        } else {
            Some(self.next_tick_time)
        }
    }

    pub fn send_to_conless(&self, addr: AddrType, msg: SendType::Conless,
            event_queue: &mut VecDeque<SocketEvent<AddrType, RecvType>>) {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        let header = MessageHeader::Conless;
        let header_size = header.pack(&mut buf).unwrap();
        let payload_size = msg.pack(&mut buf[header_size..]).unwrap();
        let msg_size = header_size + payload_size;
        if let Err(err) = self.socket.send_to(&buf[..msg_size], addr) {
            event_queue.push_back(NetworkError(err));
        }
    }

    pub fn send_to_reliable(&mut self, con_id: ConId, msg: SendType::Reliable,
            event_queue: &mut VecDeque<SocketEvent<AddrType, RecvType>>) {
        let mut buffer_full = false;
        if let Some(con) = self.connections.get_mut(&con_id) {
            if con.disconnecting {
                println!("DEBUG: Tried to send message with disconnecting connection!");
                return;
            }

            match con.send_reliable::<SendType, WrappedUdpSocketType, RecvType>(
                msg,
                &self.socket,
            ) {
                Ok(_) => (),
                Err(SendReliableError::BufferFull) => buffer_full = true,
                Err(SendReliableError::NetworkError(e)) => event_queue.push_back(NetworkError(e)),
            }
        } else {
            println!("ERROR: Tried to send reliable message without connection!");
        }

        if buffer_full {
            let con = self.connections.remove(&con_id).unwrap();
            self.con_ids_by_addr.remove(&con.addr).unwrap();
            event_queue.push_back(SocketEvent::ConnectionEnd {
                reason: TimedOut,
                con_id,
            });
        }
    }

    pub fn send_to_unreliable(&mut self, con_id: ConId, msg: SendType::Unreliable,
            event_queue: &mut VecDeque<SocketEvent<AddrType, RecvType>>) {
        if let Some(con) = self.connections.get_mut(&con_id) {
            if con.disconnecting {
                println!("DEBUG: Tried to send message with disconnecting connection!");
                return;
            }

            con.send_unreliable::<SendType, WrappedUdpSocketType, RecvType>(
                msg,
                &self.socket,
                event_queue,
            );
        } else {
            println!("ERROR: Tried to send unreliable message without connection!");
        }
    }

    pub fn broadcast_reliable(&mut self, msg: SendType::Reliable,
            event_queue: &mut VecDeque<SocketEvent<AddrType, RecvType>>) {
        for (&con_id, con) in self.connections.iter_mut() {
            if !con.disconnecting {
                // TODO pack here
                match con.send_reliable::<SendType, WrappedUdpSocketType, RecvType>(
                    msg.clone(),
                    &self.socket,
                ) {
                    Ok(_) => (),
                    Err(SendReliableError::BufferFull) => {
                        self.remove_connections_buffer.push(con_id)
                    },
                    Err(SendReliableError::NetworkError(e)) => {
                        event_queue.push_back(NetworkError(e))
                    },
                }
            }
        }

        for &con_id in self.remove_connections_buffer.iter() {
            let con = self.connections.remove(&con_id).unwrap();
            self.con_ids_by_addr.remove(&con.addr).unwrap();
            event_queue.push_back(SocketEvent::ConnectionEnd {
                reason: TimedOut,
                con_id
            });
        }
    }

    pub fn broadcast_unreliable(&mut self, msg: SendType::Unreliable,
            event_queue: &mut VecDeque<SocketEvent<AddrType, RecvType>>) {
        for (_, con) in self.connections.iter_mut() {
            if !con.disconnecting {
                // TODO pack here
                con.send_unreliable::<SendType, WrappedUdpSocketType, RecvType>(
                    msg.clone(),
                    &self.socket,
                    event_queue,
                );
            }
        }
    }

    // TODO maybe this wrapper method is not needed and we just return if the time is up
    pub fn recv_from_until<'a>(&'a mut self, until: Instant)
            -> Option<SocketEvent<AddrType, RecvType>> {
        // first make sure we read a message if there are any
        self.socket.set_nonblocking(true).unwrap();
        let result = self.recv_from(None);
        self.socket.set_nonblocking(false).unwrap();
        if let Some(_) = result {
            return result;
        }

        // if there was no message, wait for one until time out
        self.recv_from(Some(until))
    }

    // reads messages until there is a valid one or an error occurs
    // time out errors are transformed into None
    fn recv_from<'a>(&mut self, until: Option<Instant>) -> Option<SocketEvent<AddrType, RecvType>> {
        let mut buf = [0; MAX_MESSAGE_LENGTH];
        loop {
            if let Some(until) = until {
                let now = Instant::now();
                if until <= now {
                    return None;
                }
                self.socket.set_read_timeout(Some(until - now)).unwrap();
            }
            match self.socket.recv_from(&mut buf) {
                Ok((amount, addr)) => {
                    match MessageHeader::unpack(&buf[..amount]) {
                        Ok(header) => {
                            let header_size = header.packed_size().unwrap() as usize; // TODO isn't this constant?
                            let payload_slice = &buf[header_size..amount];
                            match header {
                                MessageHeader::Conless => {
                                    let con_id = self.con_ids_by_addr.get(&addr).map(|id| *id);
                                    match RecvType::Conless::unpack(payload_slice) {
                                        Ok(clmsg) => return Some(MessageReceived(Conless {
                                            addr,
                                            con_id,
                                            clmsg,
                                        })),
                                        Err(e) => println!(
                                            "DEBUG: Received malformed message. Unpack error: {:?}",
                                            e,
                                        ),
                                    }
                                },
                                MessageHeader::Conful {
                                    ack: their_ack,
                                    resend: their_resend,
                                    conful_header
                                } => {
                                    if let Some(&con_id) = self.con_ids_by_addr.get(&addr) {
                                        if let Some(event) = self.handle_conmessage(
                                            con_id,
                                            their_ack,
                                            their_resend,
                                            conful_header,
                                            payload_slice,
                                        ) {
                                            return Some(event);
                                        }
                                    } else {
                                        let mut buf = [0; MAX_MESSAGE_LENGTH];
                                        let header = MessageHeader::ConReset;
                                        let header_size = header.pack(&mut buf).unwrap();
                                        if let Err(err) = self.socket.send_to(
                                            &buf[..header_size],
                                            addr
                                        ) {
                                            return Some(NetworkError(err));
                                        }
                                        println!("DEBUG: Received connectionful message \
                                                  from unknown host!");
                                    }
                                },
                                MessageHeader::ConReset => {
                                    if let Some(con_id) = self.con_ids_by_addr.remove(&addr) {
                                        let con = self.connections.remove(&con_id).unwrap();
                                        if con.disconnecting {
                                            return Some(DisconnectingConnectionEnd {
                                                reason: Reset,
                                                con_id,
                                            });
                                        } else {
                                            return Some(ConnectionEnd {
                                                reason: Reset,
                                                con_id,
                                            });
                                        }
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
                        ErrorKind::WouldBlock | ErrorKind::TimedOut => return None,
                        _ => return Some(NetworkError(e)),
                    };
                }
            }
        }
    }

    fn handle_conmessage(
        &mut self,
        con_id: ConId,
        their_ack: u64,
        their_resend: bool,
        header: ConfulHeader,
        payload_slice: &[u8]
    ) -> Option<SocketEvent<AddrType, RecvType>> {
        {
            let con = self.connections.get_mut(&con_id).unwrap();
            con.on_ack(their_ack, their_resend);
            if con.disconnecting {
                match header {
                    ConfulHeader::Reliable(_) => {
                        println!(
                            "DEBUG: Received reliable message from disconnecting connection!"
                        );
                    },
                    ConfulHeader::Unreliable => {
                        println!(
                            "DEBUG: Received unreliable message from disconnecting connection!"
                        );
                    },
                    ConfulHeader::Ack => (),
                }
                if !con.sent_messages.is_empty() {
                    return None;
                }
            } else {
                match header {
                    ConfulHeader::Reliable(id) => {
                        if id == con.my_ack {
                            match RecvType::Reliable::unpack(payload_slice) {
                                Ok(rmsg) => {
                                    con.my_ack += 1;
                                    println!("DEBUG: Received reliable message!");
                                    return Some(MessageReceived(Conful {
                                        con_id,
                                        cmsg: Reliable::<RecvType>(rmsg),
                                    }));
                                }
                                Err(e) => println!(
                                    "DEBUG: Received malformed message. Unpack error: {:?}",
                                    e,
                                ),
                            }
                        } else if id > con.my_ack {
                            con.my_resend = true;
                            println!("DEBUG: Received early packet!");
                        } else {
                            println!("DEBUG: Received late packet!");
                        }
                    },
                    ConfulHeader::Unreliable => {
                        match RecvType::Unreliable::unpack(payload_slice) {
                            Ok(umsg) => {
                                return Some(MessageReceived(
                                    Conful::<AddrType, RecvType> {
                                        con_id,
                                        cmsg: Unreliable::<RecvType>(umsg),
                                    }
                                ));
                            },
                            Err(e) => println!(
                                "DEBUG: Received malformed message. Unpack error: {:?}", e
                            ),
                        }
                    },
                    ConfulHeader::Ack => (),
                }
                return None;
            }
        }

        let con = self.connections.remove(&con_id).unwrap();
        self.con_ids_by_addr.remove(&con.addr).unwrap();
        Some(DoneDisconnecting(con_id))
    }
}