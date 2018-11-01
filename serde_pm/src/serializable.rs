extern crate rlibc;
extern crate serde;

use std::cell::RefCell;
use std::ops::Deref;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum SerializationError {
    BoolError,
    I8Error,
    U8Error,
    I32Error,
    U32Error,
    I64Error,
    U64Error,
    StringError,
    BytesError,
    ByteArrayError,
    WrongSVUID,
    LimitExceeded,
}

impl fmt::Display for SerializationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl Error for SerializationError {
    fn description(&self) -> &str {
        match *self {
            SerializationError::BoolError => "failed to (de)serialize bool",
            SerializationError::I8Error => "failed to (de)serialize i8",
            SerializationError::U8Error => "failed to (de)serialize u8",
            SerializationError::I32Error => "failed to (de)serialize i32",
            SerializationError::U32Error => "failed to (de)serialize u32",
            SerializationError::I64Error => "failed to (de)serialize i64",
            SerializationError::U64Error => "failed to (de)serialize u64",
            SerializationError::StringError => "failed to (de)serialize String",
            SerializationError::BytesError => "failed to (de)serialize bytes",
            SerializationError::ByteArrayError => "failed to (de)serialize byte array",
            SerializationError::WrongSVUID => "wrong SVUID",
            SerializationError::LimitExceeded => "buffer limit exceeded",
        }
    }
}

impl Serializable for i32 {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) -> Result<(), SerializationError> {
        stream.write_i32(0)?;
        stream.write_i32(*self)?;
        Ok(())
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) -> Result<(), SerializationError> {
        *self = stream.read_i32()?;
        Ok(())
    }
}

impl Serializable for i64 {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) -> Result<(), SerializationError> {
        stream.write_i32(0)?;
        stream.write_i64(*self)?;
        Ok(())
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) -> Result<(), SerializationError> {
        *self = stream.read_i64()?;
        Ok(())
    }
}

impl Serializable for u32 {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) -> Result<(), SerializationError> {
        stream.write_i32(0)?;
        stream.write_u32(*self)?;
        Ok(())
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) -> Result<(), SerializationError> {
        *self = stream.read_u32()?;
        Ok(())
    }
}

impl Serializable for u64 {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) -> Result<(), SerializationError> {
        stream.write_i32(0)?;
        stream.write_u64(*self)?;
        Ok(())
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) -> Result<(), SerializationError> {
        *self = stream.read_u64()?;
        Ok(())
    }
}

impl Serializable for u8 {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) -> Result<(), SerializationError> {
        stream.write_i32(0)?;
        stream.write_byte(*self)?;
        Ok(())
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) -> Result<(), SerializationError> {
        *self = stream.read_byte()?;
        Ok(())
    }
}

impl Serializable for bool {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) -> Result<(), SerializationError> {
        stream.write_i32(0)?;
        stream.write_bool(self.clone())?;
        Ok(())
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) -> Result<(), SerializationError> {
        *self = stream.read_bool()?;
        Ok(())
    }
}

impl Serializable for String {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) -> Result<(), SerializationError> {
        stream.write_i32(0)?;
        stream.write_string(self.clone())?;
        Ok(())
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) -> Result<(), SerializationError> {
        *self = stream.read_string()?;
        Ok(())
    }
}

pub struct SerializedBuffer {
    pub buffer: Vec<u8>,
    position: usize,
    limit: usize,
    capacity: usize,
    calculated_size_only: bool,
}

impl SerializedBuffer {
    pub fn new_with_size(size: usize) -> Self {
        let sb = SerializedBuffer {
            buffer: vec![0u8; size],
            position: 0,
            limit: size,
            capacity: size,
            calculated_size_only: false,
        };
        sb
    }

    pub fn new(calculate: bool) -> Self {
        let sb = SerializedBuffer {
            buffer: Vec::new(),
            position: 0,
            limit: 0,
            capacity: 0,
            calculated_size_only: calculate,
        };
        sb
    }

    pub fn from_slice(buff: &[u8]/*, length: usize*/) -> Self {
        let length = buff.len();
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

    pub fn has_remaining(&self) -> bool {
        return self.position > self.limit;
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

    pub fn compact(&mut self) {
        use std::mem::size_of;

        if self.position == self.limit {
            return;
        }
        let buffer_ptr = &mut self.buffer[0] as *mut u8;

        unsafe {
            rlibc::memmove(buffer_ptr, buffer_ptr.offset(self.position as isize), size_of::<u8>() * (self.limit - self.position));
        }

        self.position = self.limit - self.position;
        self.limit = self.capacity;
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

    pub fn reuse(&self) {

    }

    pub fn write_byte(&mut self, i: u8) -> Result<(), SerializationError> {
        if !self.calculated_size_only {
            if self.position + 1 > self.limit {
                return Err(SerializationError::U8Error);
            }
            self.buffer[self.position] = i as u8;
            self.position += 1;
        } else {
            self.capacity += 1;
        }
        Ok(())
    }

    pub fn write_i32(&mut self, i: i32) -> Result<(), SerializationError> {
        if !self.calculated_size_only {
            if self.position + 4 > self.limit {
                return Err(SerializationError::I32Error);
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
        Ok(())
    }

    pub fn write_i64(&mut self, i: i64) -> Result<(), SerializationError> {
        if !self.calculated_size_only {
            if self.position + 8 > self.limit {
                return Err(SerializationError::I64Error);
            }
            self.buffer[self.position] = i as u8;
            self.position += 1;
            self.buffer[self.position] = (i >> 8) as u8;
            self.position += 1;
            self.buffer[self.position] = (i >> 16) as u8;
            self.position += 1;
            self.buffer[self.position] = (i >> 24) as u8;
            self.position += 1;
            self.buffer[self.position] = (i >> 32) as u8;
            self.position += 1;
            self.buffer[self.position] = (i >> 40) as u8;
            self.position += 1;
            self.buffer[self.position] = (i >> 48) as u8;
            self.position += 1;
            self.buffer[self.position] = (i >> 56) as u8;
            self.position += 1;
        } else {
            self.capacity += 8;
        }
        Ok(())
    }

    pub fn write_u64(&mut self, i: u64) -> Result<(), SerializationError> {
        self.write_i64(i as i64)
    }

    pub fn write_u32(&mut self, i: u32) -> Result<(), SerializationError> {
        self.write_i32(i as i32)
    }

    pub fn write_bool(&mut self, val: bool) -> Result<(), SerializationError> {
        if !self.calculated_size_only {
            if val {
                self.write_i32(0x6e4a64b3)?;
            } else {
                self.write_i32(0x3f5d29c3)?;
            }
        } else {
            self.capacity += 4;
        }
        Ok(())
    }

    fn write_bytes_internal(&mut self, b:&[u8], length:usize) -> Result<(), SerializationError> {
        if length == 0 {
            return Ok(());
        }

        use std::mem::size_of;
        let buffer_ptr = &mut self.buffer[0] as *mut u8;
        let b_ptr = &b[0] as *const u8;

        unsafe {
            rlibc::memcpy(buffer_ptr.offset(self.position as isize), b_ptr, size_of::<u8>() * length);
        }
        self.position += length;
        Ok(())
    }

    pub fn write_bytes(&mut self, b:&[u8]/*, length:usize*/) -> Result<(), SerializationError> {
        let length = b.len();
        if !self.calculated_size_only {
            if self.position + length > self.limit {
                return Err(SerializationError::BytesError);
            }
            self.write_bytes_internal(b, length)?;
        } else {
            self.capacity += length;
        }
        Ok(())
    }

    pub fn write_bytes_offset(&mut self, b:&[u8], length:usize) -> Result<(), SerializationError> {
        if !self.calculated_size_only {
            if self.position + length > self.limit {
                return Err(SerializationError::BytesError);
            }
            self.write_bytes_internal(b, length)?;
        } else {
            self.capacity += length;
        }
        Ok(())
    }

    pub fn write_bytes_serialized_buffer(&mut self, b: &mut SerializedBuffer) -> Result<(), SerializationError> {
        let length = b.limit() - b.position();
        if length == 0 {
            return Ok(());
        }
        if !self.calculated_size_only {
            if self.position + length > self.limit {
                return Err(SerializationError::ByteArrayError);
            }
            self.write_bytes_internal(&b.buffer[b.position()..], length)?;
            let lim = b.limit();
            b.set_position(lim);
        } else {
            self.capacity += length;
        }
        Ok(())
    }

    pub fn write_byte_array(&mut self, b: &[u8]) -> Result<(), SerializationError> {
        let length = b.len();

        if length <= 253 {
            if !self.calculated_size_only {
                if self.position + 1 > self.limit {
                    return Err(SerializationError::ByteArrayError);
                }
                self.buffer[self.position] = length as u8;
                self.position += 1;
            } else {
                self.capacity += 1;
            }
        } else {
            if !self.calculated_size_only {
                if self.position + 4 > self.limit {
                    return Err(SerializationError::ByteArrayError);
                }
                self.buffer[self.position] = 254u8;
                self.position += 1;
                self.buffer[self.position] = length as u8;
                self.position += 1;
                self.buffer[self.position] = (length >> 8) as u8;
                self.position += 1;
                self.buffer[self.position] = (length >> 16) as u8;
                self.position += 1;
            } else {
                self.capacity += 4;
            }
        }

        if !self.calculated_size_only {
            if self.position + length > self.limit {
                return Err(SerializationError::ByteArrayError);
            }

            self.write_bytes_internal(b, length)?;
        } else {
            self.capacity += length;
        }

        let mut addition = (length + (if length <= 253 { 1 } else { 4 })) % 4;

        if addition != 0 {
            addition = 4 - addition;
        }

        if !self.calculated_size_only && self.position + addition > self.limit {
            return Err(SerializationError::ByteArrayError);
        }

        for _ in 0..addition {
            if !self.calculated_size_only {
                self.buffer[self.position] = 0u8;
                self.position += 1;
            } else {
                self.capacity += 1;
            }
        }
        Ok(())
    }

    pub fn write_byte_array_serialized_buffer(&mut self, b: &mut SerializedBuffer) -> Result<(), SerializationError> {
        b.rewind();
        self.write_byte_array(&b.buffer)
    }

    pub fn write_f32(&mut self, f: f32) -> Result<(), SerializationError> {
        self.write_u32(f32::to_bits(f))
    }

    pub fn write_f64(&mut self, f: f64) -> Result<(), SerializationError> {
        self.write_u64(f64::to_bits(f))
    }

    pub fn write_string(&mut self, s:String) -> Result<(), SerializationError> {
        self.write_byte_array(s.as_ref())
    }

    pub fn read_i32(&mut self) -> Result<i32, SerializationError> {
        if self.position + 4 > self.limit {
            return Err(SerializationError::I32Error);
        }
        let result =
            ((self.buffer[self.position] as i32 & 0xff) |
                ((self.buffer[self.position + 1] as i32 & 0xff) << 8) |
                ((self.buffer[self.position + 2] as i32 & 0xff) << 16) |
                ((self.buffer[self.position + 3] as i32 & 0xff) << 24)) as i32;
        self.position += 4;
        Ok(result)
    }

    pub fn read_i64(&mut self) -> Result<i64, SerializationError> {
        if self.position + 8 > self.limit {
            return Err(SerializationError::I64Error);
        }

        let result =
            ((self.buffer[self.position] as i64 & 0xff) |
                ((self.buffer[self.position + 1] as i64 & 0xff) << 8) |
                ((self.buffer[self.position + 2] as i64 & 0xff) << 16) |
                ((self.buffer[self.position + 3] as i64 & 0xff) << 24) |
                ((self.buffer[self.position + 4] as i64 & 0xff) << 32) |
                ((self.buffer[self.position + 5] as i64 & 0xff) << 40) |
                ((self.buffer[self.position + 6] as i64 & 0xff) << 48) |
                ((self.buffer[self.position + 7] as i64 & 0xff) << 56)) as i64;
        self.position += 8;
        Ok(result)
    }

    pub fn read_u32(&mut self) -> Result<u32, SerializationError> {
        self.read_i32().map(|i| i as u32)
    }

    pub fn read_u64(&mut self) -> Result<u64, SerializationError> {
        self.read_i64().map(|i| i as u64)
    }

    pub fn read_f32(&mut self) -> Result<f32, SerializationError> {
        let i = self.read_u32()?;
        Ok(f32::from_bits(i))
    }

    pub fn read_f64(&mut self) -> Result<f64, SerializationError> {
        let i = self.read_u64()?;
        Ok(f64::from_bits(i))
    }

    pub fn read_byte(&mut self) -> Result<u8, SerializationError> {
        if self.position + 1 > self.limit {
            return Err(SerializationError::U8Error);
        }
        let result = self.buffer[self.position];
        self.position += 1;
        Ok(result)
    }

    pub fn read_bool(&mut self) -> Result<bool, SerializationError> {
        let i = self.read_u32()?;

        if i == 0x6e4a64b3 {
            return Ok(true);
        } else if i == 0x3f5d29c3 {
            return Ok(false);
        }

        Err(SerializationError::BoolError)
    }

    pub fn read_string(&mut self) -> Result<String, SerializationError> {
        let mut sl = 1usize;
        if self.position + 1 > self.limit {
            return Err(SerializationError::StringError);
        }
        let mut l = self.buffer[self.position] as usize;
        self.position += 1;
        if l >= 254 {
            if self.position + 3 > self.limit {
                return Err(SerializationError::StringError);
            }
            l = (self.buffer[self.position] as usize) | ((self.buffer[self.position + 1] as usize) << 8) | ((self.buffer[self.position + 2] as usize) << 16);
            self.position += 3;
            sl = 4;
        }
        let mut addition = (l + sl) % 4 as usize;
        if addition != 0 {
            addition = 4 - addition;
        }
        if self.position + l + addition > self.limit {
            return Err(SerializationError::StringError);
        }

        let result = String::from_utf8(self.buffer[self.position..(self.position+l)].to_vec()).unwrap_or(String::from(""));
        self.position += l + addition;
        Ok(result)
    }

    pub fn read_byte_array(&mut self) -> Result<Vec<u8>, SerializationError> {
        let mut sl = 1usize;
        if self.position + 1 > self.limit {
            return Err(SerializationError::ByteArrayError);
        }
        let mut l = self.buffer[self.position] as usize;
        self.position += 1;

        if l >= 254 {
            if self.position + 3 > self.limit {
                return Err(SerializationError::ByteArrayError);
            }
            l = (self.buffer[self.position] as usize) | ((self.buffer[self.position + 1] as usize) << 8) | ((self.buffer[self.position + 2] as usize) << 16);
            self.position += 3;
            sl = 4;
        }
        let mut addition = (l + sl) % 4 as usize;
        if addition != 0 {
            addition = 4 - addition;
        }
        if self.position + l + addition > self.limit {
            return Err(SerializationError::ByteArrayError);
        }

        let mut result = vec![0u8; l];
        result.copy_from_slice(&self.buffer[self.position..(self.position + l)]);
        self.position += l + addition;
        Ok(result)
    }

    pub fn read_bytes(&mut self, b: &mut [u8], length: usize) -> Result<(), SerializationError> {
        if self.position + length > self.limit {
            return Err(SerializationError::BytesError);
        }

        use std::mem::size_of;
        let buffer_ptr = &self.buffer[0] as *const u8;
        let b_ptr = &mut b[0] as *mut u8;

        unsafe {
            rlibc::memcpy(b_ptr, buffer_ptr.offset(self.position as isize), size_of::<u8>() * length);
        }

        self.position += length;
        Ok(())
    }
}

impl Clone for SerializedBuffer {
    fn clone(&self) -> Self {
        let bytes = self.buffer.clone();
        let len = bytes.len();
        let mut buffer = SerializedBuffer::new_with_size(len);
        buffer.buffer = bytes;
        buffer.position = self.position;
        buffer.limit = self.limit;
        buffer.capacity = self.capacity;
        buffer.calculated_size_only = self.calculated_size_only;
        buffer
    }
}

impl Deref for SerializedBuffer {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.buffer[self.position..self.limit]
    }
}

pub trait Serializable : Default {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) -> Result<(), SerializationError>;
    fn read_params(&mut self, stream: &mut SerializedBuffer) -> Result<(), SerializationError>;
}

thread_local! {
    pub static sizeCalculatorBuffer: RefCell<SerializedBuffer> = RefCell::new(SerializedBuffer::new(true));
}

pub fn calculate_object_size<T>(packet: &T) -> usize where T: Serializable {
    let mut capacity = 0usize;

    sizeCalculatorBuffer.with(|f| {
        let mut b = f.try_borrow_mut().unwrap();
        b.clear_capacity();
        packet.serialize_to_stream(&mut b).expect("Failed to calculate object size");
        capacity = b.capacity();
    });

    capacity
}

pub fn get_serialized_object<T>(packet: &T, with_svuid: bool) -> Result<SerializedBuffer, SerializationError> where T: Serializable {
    let size = calculate_object_size(packet);
    let mut sb = SerializedBuffer::new_with_size(size);
    packet.serialize_to_stream(&mut sb)?;

    if !with_svuid {
        sb.set_position(4);
    }

    Ok(sb)
}
