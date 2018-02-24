extern crate byteorder;
extern crate mio;
extern crate rand;
extern crate slab;

#[macro_use] extern crate log;
extern crate env_logger;

pub mod network;
pub mod model;
pub mod storage;

use std::io::prelude::*;
use std::net::{TcpStream, SocketAddr, IpAddr};
use std::thread;
use std::time::Duration;
use std::str::from_utf8;
use byteorder::{ByteOrder, BigEndian};
use network::packet::{SerializedBuffer, Packet, get_object_size};
use network::rpc::KeepAlive;
use model::config::PORT;

static NTHREADS: i32 = 1;

pub fn send_packet<T>(stream: &mut TcpStream, packet: T, message_id : i64) where T: Packet {
    let message_length = get_object_size(&packet);
    let size = 8 + 4 + message_length as usize;
    println!("{:X}", size);
    println!("{:b}", size);
    let mut buffer = SerializedBuffer::new_with_size(size);
    buffer.set_position(0);
    buffer.write_i64(message_id);
    buffer.write_i32(message_length as i32);
    packet.serialize_to_stream(&mut buffer);
    send_data(stream, buffer);
}

pub fn send_data(stream: &mut TcpStream, mut buff: SerializedBuffer) {
    buff.rewind();

    let mut buffer_len = 0;
    let mut packet_length = (buff.limit() / 4) as i32;

    if packet_length < 0x7f {
        buffer_len += 1;
    } else {
        buffer_len += 4;
    }

    let mut buffer = SerializedBuffer::new_with_size(buffer_len);
    if packet_length < 0x7f {
        buffer.write_byte(packet_length as u8);
    } else {
        packet_length = (packet_length << 8) + 0x7f;
        buffer.write_i32(packet_length);
    }


    println!("total len = {}", buff.limit() + buffer.limit());
    buffer.rewind();
    stream.write_all(&buffer.buffer[0..buffer.limit()]);
    buff.rewind();
    stream.write_all(&buff.buffer[0..buff.limit()]);
}

fn main() {
//    for i in 0..NTHREADS {
//        let _ = thread::spawn(move|| {
//            let host = "127.0.0.1".parse::<IpAddr>().expect("Failed to parse host string");
//            let addr = SocketAddr::new(host, 44832);
//            let mut stream = TcpStream::connect(addr).expect("Couldn't not connect to neighbor"); //.unwrap();
//
//            loop {
//                let msg = format!("the answer is {}", i);
//                let mut buf = [0u8; 8];
//
//                println!("thread {}: Sending over message length of {}", i, msg.len());
//                BigEndian::write_u64(&mut buf, msg.len() as u64);
//                stream.write_all(buf.as_ref()).unwrap();
//                stream.write_all(msg.as_ref()).unwrap();
//
//                let mut buf = [0u8; 8];
//                stream.read(&mut buf).unwrap();
//
//                let msg_len = BigEndian::read_u64(&mut buf);
//                println!("thread {}: Reading message length of {}", i, msg_len);
//
//                let mut r = [0u8; 32];
//                let s_ref = <TcpStream as Read>::by_ref(&mut stream);
//
//                match s_ref.take(msg_len).read(&mut r) {
//                    Ok(0) => {
//                        println!("thread {}: 0 bytes read", i);
//                    },
//                    Ok(n) => {
//                        println!("thread {}: {} bytes read", i, n);
//
//                        let s = from_utf8(&r[..]).unwrap();
//                        println!("thread {} read = {}", i, s);
//                    },
//                    Err(e) => {
//                        panic!("thread {}: {}", i, e);
//                    }
//                }
//                thread::sleep(Duration::from_secs(1));
//            }
//        });
//    }

    let host = "127.0.0.1".parse::<IpAddr>().expect("Failed to parse host string");
    let addr = SocketAddr::new(host, PORT);
    let mut stream = TcpStream::connect(addr).expect("Couldn't not connect to neighbor"); //.unwrap();

    let packet = KeepAlive {};
    send_packet(&mut stream, packet, 1337);
    thread::sleep_ms(500);
    let mut buf = [0u8; 16];
    stream.take(16).read(&mut buf).unwrap();
    println!("{:?}", buf);
    loop {}
}
