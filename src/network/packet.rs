use byteorder::{ByteOrder, BigEndian};
use std::cell::RefCell;

pub struct SerializedBuffer {
    buffer: Vec<u8>,
    position: usize,
    limit: usize,
    capacity: usize,
    calculated_size_only: bool,

}

impl SerializedBuffer {
    pub fn new_with_size(size: usize) -> SerializedBuffer {
        let mut sb = SerializedBuffer {
            buffer: Vec::with_capacity(size),
            position: 0,
            limit: size,
            capacity: size,
            calculated_size_only: false,
        };
        unsafe { sb.buffer.set_len(size); }
        sb
    }

    pub fn new(calculate: bool) -> SerializedBuffer {
        let sb = SerializedBuffer {
            buffer: Vec::new(),
            position: 0,
            limit: 0,
            capacity: 0,
            calculated_size_only: calculate,
        };
        sb
    }

    pub fn new_with_buffer(buff: &[u8], length: usize) -> SerializedBuffer {
        SerializedBuffer {
            buffer: Vec::from(buff),
            position: 0,
            limit: length,
            capacity: length,
            calculated_size_only: false,
        }
    }

    pub fn set_position(&mut self, pos: usize) {
        if self.position > self.limit {
            return;
        }
        self.position = pos;
    }

    pub fn set_limit(&mut self, limit: usize) {
        if limit > self.capacity {
            return;
        }

        if self.position > limit {
            self.position = limit;
        }

        self.limit = limit;
    }

    pub fn flip(&mut self) {
        self.limit = self.position;
        self.position = 0;
    }

    pub fn clear(&mut self) {
        self.limit = self.capacity;
        self.position = 0;
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn buffer(&mut self) -> &mut [u8] {
        self.buffer.as_mut()
    }

    pub fn limit(&self) -> usize {
        self.limit
    }

    pub fn remaining(&self) -> usize {
        self.limit - self.position
    }

    pub fn clear_capacity(&mut self) {
        if !self.calculated_size_only {
            return;
        }

        self.capacity = 0;
    }

    pub fn rewind(&mut self) {
        self.position = 0;
    }

    pub fn compact(&self) {
        if self.position == self.limit {
            return;
        }
        // TODO
    }

    pub fn skip(&mut self, len: usize) {
        if !self.calculated_size_only {
            if self.position + len > self.limit {
                return;
            }
            self.position += len;
        } else {
            self.capacity += len;
        }
    }

    pub fn write_i32(&mut self, i: i32) {
        if !self.calculated_size_only {
            if self.position + 4 > self.limit {
//                if error != nullptr {
//                    *error = true;
//                }
                panic!("write int32 error");
                return;
            }
            self.buffer[self.position] = i as u8;
            self.position += 1;
            self.buffer[self.position] = (i >> 8) as u8;
            self.position += 1;
            self.buffer[self.position] = (i >> 16) as u8;
            self.position += 1;
            self.buffer[self.position] = (i >> 24) as u8;
            self.position += 1;
        } else {
            self.capacity += 4;
        }
    }

    pub fn read_i32(&mut self) -> i32 {
        if self.position + 4 > self.limit {
            panic!("read int32 error!");
        }
        let result =
            ((self.buffer[self.position] as i32 & 0xff) |
            ((self.buffer[self.position + 1] as i32 & 0xff) << 8) |
            ((self.buffer[self.position + 2] as i32 & 0xff) << 16) |
            ((self.buffer[self.position + 3] as i32 & 0xff) << 24)) as i32;
        self.position += 4;
        result
    }
}

pub trait Packet {
    fn read_params(&self, stream: &SerializedBuffer, error: bool);
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer);
}

thread_local! {
    pub static sizeCalculatorBuffer: RefCell<SerializedBuffer> = RefCell::new(SerializedBuffer::new(true));
}

fn calcCapacity<T>(packet: &mut T) where T: Packet {
//    sizeCalculatorBuffer.try_borrow_mut().unwrap().clear_capacity();
//    serializeToStream(sizeCalculatorBuffer);
//    packet.serialize_to_stream(sizeCalculatorBuffer);
//    return sizeCalculatorBuffer->capacity();
}

//    pub fn deserializeResponse(stream: SerializedBuffer, constructor: usize) -> mut& Packet {
//        Packet
//    }

//     pub fn getObjectSize() -> usize {
//        sizeCalculatorBuffer->clearCapacity();
//        serializeToStream(sizeCalculatorBuffer);
//        sizeCalculatorBuffer->capacity();
//    }
