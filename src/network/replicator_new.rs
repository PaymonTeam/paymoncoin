//#![deny(warnings)]

extern crate tokio;
extern crate bytes;
extern crate tokio_codec;

use self::tokio::io;
use self::tokio::net::{TcpListener, TcpStream};
use self::tokio::prelude::*;
use self::tokio_codec::Decoder;
use self::tokio::timer::Interval;
use self::bytes::BytesMut;
use self::codec::Bytes;

use std::net::{SocketAddr, IpAddr};
use std::io::{Read, Write};
use std::collections::HashMap;
use std::iter;
use std::env;
use std::io::{BufReader, BufRead};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Instant, Duration};
use std::thread;
use network::Node;
use model::config::{Configuration, ConfigurationSettings, PORT};
use futures::sync::mpsc;
use futures::sync::mpsc::{Sender, UnboundedSender};
use crossbeam::scope;

use utils::AM;
use network::Neighbor;
use network::packet::SerializedBuffer;

pub enum ReadState<A> {
    ReadingData {
        len: u32,
        current_read: u32,
        buf: Vec<u8>,
        f_b: u8,
        a: A,
        len_read: bool
    },
    Empty,
}

pub struct ReadPacket<A> {
    pub state: ReadState<A>
}

pub fn read_packet<A>(a: A, buf: Vec<u8>) -> ReadPacket<A>
    where A: AsyncRead + BufRead, {
    ReadPacket {
        state: ReadState::ReadingData {
            a,
            buf,
            len: 0,
            current_read: 0,
            f_b: 0,
            len_read: false
        }
    }
}

impl<A> Future for ReadPacket<A>
    where A: AsyncRead + BufRead
{
    type Item = (A, Vec<u8>);
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(A, Vec<u8>), io::Error> {
        use std::mem;
        use std::io::ErrorKind;
        use self::tokio::io::ReadHalf;
        use byteorder::{ByteOrder, NativeEndian, BigEndian, LittleEndian};
        use futures::prelude::*;

        let mut f_byte_buf = [0u8; 1];

        match self.state {
            ReadState::ReadingData {ref mut a, ref mut buf, ref mut len, ref mut current_read,
                ref mut f_b, ref mut len_read} => {
                if *current_read == 0 && !*len_read {
                    match a.read_exact(&mut f_byte_buf) {
                        Ok(t) => t,
                        Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                            return Ok(Async::NotReady);
                        }
                        Err(e) => return Err(e.into()),
                    };

                    if f_byte_buf.len() == 0 {
                        return Ok(Async::NotReady);
                    }

                    *current_read = 1;

                    let f_byte = f_byte_buf[0];
                    *f_b = f_byte;

                    if f_byte != 0x7f {
                        *len = (f_byte_buf[0] as u32) * 4;
                        *len_read = true;
                    } else {
                        let mut buf = [0u8; 4];
                        buf[0] = f_byte;
                        match a.read_exact(&mut buf[1..]) {
                            Ok(t) => t,
                            Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                                return Ok(Async::NotReady);
                            }
                            Err(e) => return Err(e.into()),
                        };
                        let msg_len = NativeEndian::read_u32(buf.as_ref());
                        *len = (msg_len >> 8) * 4;
                        *current_read = 4;
                        *len_read = true;
                    }
                }

                if *current_read == 1 && !*len_read {
                    let mut buf = [0u8; 4];
                    buf[0] = f_b.clone();
                    match a.read_exact(&mut buf[1..]) {
                        Ok(t) => t,
                        Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                            return Ok(Async::NotReady);
                        }
                        Err(e) => return Err(e.into()),
                    };
                    let msg_len = NativeEndian::read_u32(buf.as_ref());
                    *len = (msg_len >> 8) * 4;
                    *current_read = 4;
                    *len_read = true;
                }

                if *len_read {
                    debug!("read len={}", *len);
                    let mut nbuf = vec![0u8; *len as usize];
                    match a.read_exact(&mut nbuf) {
                        Ok(t) => t,
                        Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                            return Ok(Async::NotReady);
                        }
                        Err(e) => return Err(e.into()),
                    };
                    *buf = nbuf;
                } else {
                    return Ok(Async::NotReady);
                }
            }
            ReadState::Empty => panic!("poll ReadPacket after it's done"),
        };

        match mem::replace(&mut self.state, ReadState::Empty) {
            ReadState::ReadingData { a, buf, .. } => Ok((a, buf).into()),
            ReadState::Empty => unreachable!(),
        }
    }
}

pub struct ReplicatorSink {
//    sock: TcpStream,
    pub tx: UnboundedSender<Vec<u8>>
}

pub struct ReplicatorSource {
//    sock: TcpStream,
}

pub struct ReplicatorNew {
    node: Weak<Mutex<Node>>,
//    node_rx: Receiver<()>,
    addr: SocketAddr,

}

impl ReplicatorNew {
    pub fn new(config: &Configuration, node: Weak<Mutex<Node>>) -> Self {
        let host = "127.0.0.1".parse::<IpAddr>().expect("Failed to parse host string");
        let port = config.get_int(ConfigurationSettings::Port).unwrap_or(PORT as i32) as u16;
        let addr = SocketAddr::new(host, port);

        ReplicatorNew {
            node,
            addr,
        }
    }

    pub fn run(&mut self) {
        let addr = self.addr.clone();
        let node = self.node.clone();

        let worker = stream::iter_ok::<_, io::Error>(iter::repeat(())).take(1).for_each(move |j| {
            let socket = TcpListener::bind(&addr).unwrap();
            info!("Listening on: {}", addr);

            let connections = Arc::new(Mutex::new(HashMap::new()));

            let neighbors;
            let node_c2 = node.clone();

            if let Some(arc) = node.upgrade() {
                if let Ok(node) = arc.try_lock() {
                    neighbors = node.neighbors.clone();
                } else {
                    return Err(io::Error::new(io::ErrorKind::NotFound, "node not found"))
                }
            } else {
                return Err(io::Error::new(io::ErrorKind::NotFound, "node not found"))
            }

            let neighbors_c = neighbors.clone();

            let connector = Interval::new(Instant::now(), Duration::from_millis(5000)).for_each(move |instant| {
                let mut neighbors = neighbors.lock().unwrap();
                for n in neighbors.iter_mut() {
                    let mut neighbor_c = n.clone();
                    let mut neighbor_cc = n.clone();
                    let mut neighbor = n.lock().unwrap();
                    if neighbor.sink.is_none() && !neighbor.connecting {
                        neighbor.connecting = true;

                        let (stdin_tx, stdin_rx) = mpsc::unbounded();

                        let stdin_rx = stdin_rx.map_err(|_| panic!()); // errors not possible on rx
                        let stdout = tcp::connect(&neighbor.addr,
                                                  Box::new(stdin_rx),
                                                  neighbor_c,
                                                  stdin_tx);

                        tokio::spawn({
                            stdout.for_each(move |chunk| {
                                Ok(())
                            })
                                .map_err(move |e| {
                                    let mut neighbor = neighbor_cc.lock().unwrap();
                                    neighbor.sink = None;
                                    neighbor.source = None;
                                    neighbor.connecting = false;
                                    error!("error reading stdout; error = {:?}", e)
                                })
                        });
                    }
                }

                Ok(())
            }).map_err(|e| panic!("interval error reading stdoutrrored; err={:?}", e));

            let srv = socket.incoming()
                .map_err(|e| error!("failed to accept socket; error = {:?}", e))
                .for_each(move |stream| {
                    let node_c3 = node_c2.clone();

                    let addr = stream.peer_addr().unwrap();
                    let addr_c = addr.clone();
                    info!("New Connection: {}", addr);

                    let (reader, writer) = stream.split();

                    let (tx, rx) = mpsc::unbounded::<Vec<u8>>();
                    connections.lock().unwrap().insert(addr, tx);

                    let mut founded_neighbor: Option<AM<Neighbor>> = None;

                    if let Ok(mut neighbors) = neighbors_c.lock() {
                        for neighbor_arc in neighbors.iter() {
                            if let Ok(neighbor) = neighbor_arc.lock() {
                                if addr.ip() == neighbor.addr.ip() {
                                    founded_neighbor = Some(neighbor_arc.clone());
                                    break;
                                }
                            }
                        }

                        if let Some(n) = founded_neighbor {
                            if let Ok(ref mut neighbor) = n.lock() {
                                if neighbor.source.is_none() {
                                    neighbor.source = Some(ReplicatorSource {

                                    });
                                    //self.add_replicator(sock);
                                } else {
                                    warn!("Neighbor source already exists");
                                }
                            }
                        } else {
                            debug!("Unknown neighbor connected");
                            neighbors.push(Arc::new(Mutex::new(Neighbor::from_replicator_source(ReplicatorSource {}, addr.clone()))))
                        }
                    }

                    let connections_inner = connections.clone();
                    let reader = BufReader::new(reader);

                    let iter = stream::iter_ok::<_, io::Error>(iter::repeat(()));

                    let socket_reader = iter.fold(reader, move |reader, _| {
                        let line = read_packet(reader, Vec::new());
                        let line = line.and_then(|(reader, vec)| {
                            if vec.len() == 0 {
                                Err(io::Error::new(io::ErrorKind::BrokenPipe, "broken pipe"))
                            } else {
                                Ok((reader, vec))
                            }
                        });

                        let node_c = node_c3.clone();
                        line.map(move |(reader, vec)| {
                            if let Some(arc) = node_c.upgrade() {
                                if let Ok(mut node) = arc.try_lock() {
                                    node.on_connection_data_received(SerializedBuffer::from_slice(&vec), addr_c);
                                }
                            }
                            reader
                        })
                    });

                    let socket_writer = rx.fold(writer, |writer, msg| {
                        let amt = io::write_all(writer, msg.clone()/*.into_bytes()*/);
                        let amt = amt.map(|(writer, _)| writer);
                        amt.map_err(|_| ())
                    });

                    let connections = connections.clone();
                    let socket_reader = socket_reader.map_err(|_| ());
                    let connection = socket_reader.map(|_| ()).select(socket_writer.map(|_| ()));

//                    let connection_jh = scope(|scope| {
                        tokio::spawn(connection.then(move |_| {
                            connections.lock().unwrap().remove(&addr);
                            info!("Connection {} closed.", addr);
                            Ok(())
                        }));
//                    });

                    Ok(())
                })
//              .map_err(|e| println!("failed to accept socket; error = {:?}", e))
            ;

            tokio::spawn(connector);
            tokio::spawn(srv);
            Ok(())
        }).map_err(|e| panic!("worker error={:?}", e));

        tokio::run(worker);
    }
}


mod tcp {
    use super::tokio;
    use super::tokio_codec::Decoder;
    use super::tokio::net::TcpStream;
    use super::tokio::prelude::*;

    use super::bytes::BytesMut;
    use super::codec::Bytes;

    use std::io;
    use std::net::SocketAddr;
    use std::sync::{Mutex, Arc};
    use super::{ReplicatorSink};
    use network::Neighbor;
    use futures::sync::mpsc::UnboundedSender;

    pub fn connect(addr: &SocketAddr,
                   stdin: Box<Stream<Item = Vec<u8>, Error = io::Error> + Send>,
                   neighbor: Arc<Mutex<Neighbor>>,
                   tx: UnboundedSender<Vec<u8>>)
                   -> Box<Stream<Item = BytesMut, Error = io::Error> + Send>
    {
        let tcp = TcpStream::connect(addr);

        Box::new(tcp.map(move |stream| {
            info!("Connected");
            {
                let mut neighbor = neighbor.lock().unwrap();
                neighbor.sink = Some(ReplicatorSink {
                    tx
                });
                neighbor.connecting = false;
            }

            let (sink, stream) = Bytes.framed(stream).split();//stream.split();//
            let mut neighbor_c = neighbor.clone();
            tokio::spawn(stdin.forward(sink).then(move |result| {
                if let Err(e) = result {
                    let mut neighbor = neighbor_c.lock().unwrap();
                    neighbor.sink = None;
                    neighbor.source = None;
                    neighbor.connecting = false;
                    panic!("failed to write to socket: {}", e)
                }
                Ok(())
            }));

            stream
        }).flatten_stream())
    }
}

mod udp {
    use std::io;
    use std::net::SocketAddr;

    use super::tokio;
    use super::tokio::net::{UdpSocket, UdpFramed};
    use super::tokio::prelude::*;
    use super::bytes::BytesMut;

    use super::codec::Bytes;

    pub fn connect(&addr: &SocketAddr,
                   stdin: Box<Stream<Item = Vec<u8>, Error = io::Error> + Send>)
                   -> Box<Stream<Item = BytesMut, Error = io::Error> + Send>
    {
        // We'll bind our UDP socket to a local IP/port, but for now we
        // basically let the OS pick both of those.
        let addr_to_bind = if addr.ip().is_ipv4() {
            "0.0.0.0:0".parse().unwrap()
        } else {
            "[::]:0".parse().unwrap()
        };
        let udp = UdpSocket::bind(&addr_to_bind)
            .expect("failed to bind socket");

        // Like above with TCP we use an instance of `Bytes` codec to transform
        // this UDP socket into a framed sink/stream which operates over
        // discrete values. In this case we're working with *pairs* of socket
        // addresses and byte buffers.
        let (sink, stream) = UdpFramed::new(udp, Bytes).split();

        // All bytes from `stdin` will go to the `addr` specified in our
        // argument list. Like with TCP this is spawned concurrently
        let forward_stdin = stdin.map(move |chunk| {
            (chunk, addr)
        }).forward(sink).then(|result| {
            if let Err(e) = result {
                panic!("failed to write to socket: {}", e)
            }
            Ok(())
        });

        // With UDP we could receive data from any source, so filter out
        // anything coming from a different address
        let receive = stream.filter_map(move |(chunk, src)| {
            if src == addr {
                Some(chunk.into())
            } else {
                None
            }
        });

        Box::new(future::lazy(|| {
            tokio::spawn(forward_stdin);
            future::ok(receive)
        }).flatten_stream())
    }
}

mod codec {
    use std::io;
    use super::bytes::{BufMut, BytesMut};
    use super::tokio_codec::{Encoder, Decoder, FramedRead, Framed, FramedParts, FramedWrite};

    /// A simple `Codec` implementation that just ships bytes around.
    ///
    /// This type is used for "framing" a TCP/UDP stream of bytes but it's really
    /// just a convenient method for us to work with streams/sinks for now.
    /// This'll just take any data read and interpret it as a "frame" and
    /// conversely just shove data into the output location without looking at
    /// it.
    pub struct Bytes;

    impl Decoder for Bytes {
        type Item = BytesMut;
        type Error = io::Error;

        fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<BytesMut>> {
            if buf.len() > 0 {
                let len = buf.len();
                Ok(Some(buf.split_to(len)))
            } else {
                Ok(None)
            }
        }
    }

    impl Encoder for Bytes {
        type Item = Vec<u8>;
        type Error = io::Error;

        fn encode(&mut self, data: Vec<u8>, buf: &mut BytesMut) -> io::Result<()> {
            buf.put(&data[..]);
            Ok(())
        }
    }
}

// Our helper method which will read data from stdin and send it along the
// sender provided.
fn read_stdin(mut tx: mpsc::Sender<Vec<u8>>) {
    let mut stdin = io::stdin();
    loop {
        let mut buf = vec![0; 1024];
        let n = match stdin.read(&mut buf) {
            Err(_) |
            Ok(0) => break,
            Ok(n) => n,
        };
        buf.truncate(n);
        tx = match tx.send(buf).wait() {
            Ok(tx) => tx,
            Err(_) => break,
        };
    }
}