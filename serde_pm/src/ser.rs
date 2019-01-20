use std::fmt;
use std::io;

use serde::ser::{self, Impossible, Serialize};
use super::error::{Error, Result, SerializationError};
use super::serializable::SerializedBuffer;
use identifiable::Identifiable;
use sized::PMSized;
use wrappers::Boxed;

use utils::{safe_uint_cast};
//use fasthash::murmur2;

pub struct Serializer {
    buff: SerializedBuffer
}

pub struct SerStruct<'a> {
    ser: &'a mut Serializer,
    len: u32,
    next_index: u32,
}

impl<'a> SerStruct<'a> {
    fn new(ser: &'a mut Serializer, len: u32) -> Self {
        SerStruct { ser, len, next_index: 0 }
    }

    fn impl_serialize_value<T>(&mut self,
                                   key: Option<&'static str>,
                                   value: &T,
                                   serializer_type: &'static str)
                                   -> Result<()>
        where T: ?Sized + Serialize
    {
        value.serialize(&mut *self.ser)
    }
}

pub struct SerNone {}

impl<'a> ser::SerializeTuple for SerStruct<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()> where
        T: Serialize {
        self.impl_serialize_value(None, value, "SerializeTuple")
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeTupleStruct for SerStruct<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()> where
        T: Serialize {
        self.impl_serialize_value(None, value, "SerializeTupleStruct")
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeTupleVariant for SerStruct<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()> where
        T: Serialize {
        self.impl_serialize_value(None, value, "SerializeTupleVariant")
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl ser::SerializeMap for SerNone {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<()> where
        T: Serialize {
        Err(SerializationError::UnserializableType.into())
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<()> where
        T: Serialize {
        Err(SerializationError::UnserializableType.into())
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for SerStruct<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()> where
        T: Serialize {
        self.impl_serialize_value(Some(key), value, "SerializeStructVariant")
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeSeq for SerStruct<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()> where
        T: Serialize {
        debug!("> ser element");
        self.impl_serialize_value(None, value, "SerializeSeq")
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeStruct for SerStruct<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()> where
        T: Serialize {
        debug!("> ser struct key={}", key);
        self.impl_serialize_value(Some(key), value, "SerializeStruct")
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = SerStruct<'a>;
    type SerializeTuple = SerStruct<'a>;
    type SerializeTupleStruct = SerStruct<'a>;
    type SerializeTupleVariant = SerStruct<'a>;
    type SerializeMap = SerNone;
    type SerializeStruct = SerStruct<'a>;
    type SerializeStructVariant = SerStruct<'a>;

    fn serialize_bool(self, v: bool) -> Result<()> {
        debug!("serialize_bool = {}", v);
        self.buff.write_bool(v)
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        debug!("serialize_i8 = {}", v);
        self.buff.write_i8(v)
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        debug!("serialize_i16 = {}", v);
        Err(SerializationError::UnserializableType.into())
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        debug!("serialize_i32 = {}", v);
        self.buff.write_i32(v)
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        debug!("serialize_i64 = {}", v);
        self.buff.write_i64(v)
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        debug!("serialize_u8 = {}", v);
        self.buff.write_u8(v)
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        debug!("serialize_u16 = {}", v);
        Err(SerializationError::UnserializableType.into())
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        debug!("serialize_u32 = {}", v);
        self.buff.write_u32(v)
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        debug!("serialize_u64 = {}", v);
        self.buff.write_u64(v)
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        debug!("serialize_f32 = {}", v);
        self.buff.write_f32(v)
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        debug!("serialize_f64 = {}", v);
        self.buff.write_f64(v)
    }

    fn serialize_char(self, v: char) -> Result<()> {
        debug!("serialize_char = {}", v);
        self.buff.write_u32(v as u32)
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        debug!("serialize_str = {}", v);
        self.buff.write_string(v)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        debug!("ser bytes {}, {:?}", v.len(), v);
        self.buff.write_byte_array(v)
    }

    fn serialize_none(self) -> Result<()> {
        Err(SerializationError::UnserializableType.into())
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<()> where
        T: Serialize {
        debug!("serialize_some");
        Err(SerializationError::UnserializableType.into())
    }

    fn serialize_unit(self) -> Result<()> {
        debug!("serialize_unit");
        Err(SerializationError::UnserializableType.into())
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<()> {
        debug!("serialize_unit_struct");
        Err(SerializationError::UnserializableType.into())
    }

    fn serialize_unit_variant(self, name: &'static str, variant_index: u32, variant: &'static str) -> Result<()> {
        debug!("serialize_unit_variant n={} i={} v={}", name, variant_index, variant);
        // TODO: make safe cast
        self.buff.write_u8(variant_index as u8)
    }

    fn serialize_newtype_struct<T: ?Sized>(self, name: &'static str, value: &T) -> Result<()> where
        T: Serialize {
        debug!("ser struct {}", name);
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(self, name: &'static str, variant_index: u32, variant: &'static str, value: &T) -> Result<()> where
        T: Serialize {
        debug!("serialize_newtype_variant {}[{}]:{}", name, variant_index, variant);
        // TODO: make safe cast
//        let hash = murmur2::hash32(variant[variant_index]);
//        self.buff.write_u32(hash);
        self.buff.write_u8(variant_index as u8);
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        debug!("ser seq len={:?}", len);

        match len {
            Some(len) => {
                self.buff.write_u32(len as u32)?;
                Ok(SerStruct::new(self, len as u32))
            },
            None => Err(SerializationError::UnserializableType.into())
        }
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        debug!("serialize_tuple limit={} len={}", /*&self.buff.buffer, */self.buff.limit(), len);
        Ok(SerStruct::new(self, safe_uint_cast(len)?))
    }

    fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeTupleStruct> {
        debug!("serialize_tuple_struct name={} len={}", name, len);
        Ok(SerStruct::new(self, safe_uint_cast(len)?))
    }

    fn serialize_tuple_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeTupleVariant> {
        debug!("serialize_tuple_variant name={} index={} variant={} len={}", name, variant_index, variant, len);
        self.buff.write_u8(variant_index as u8);
        Ok(SerStruct::new(self, safe_uint_cast(len)?))
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        debug!("serialize_map len={:?}", len);
        Err(SerializationError::UnserializableType.into())
    }

    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        debug!("serialize_struct name={}, len={}", name, len);
        Ok(SerStruct::new(self, len as u32))
    }

    fn serialize_struct_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeStructVariant> {
        debug!("serialize_struct_variant name={} index={} variant={} len={}", name, variant_index, variant, len);

        self.buff.write_u8(variant_index as u8);

        Err(SerializationError::UnserializableType.into())
    }

    fn collect_str<T: ?Sized>(self, value: &T) -> Result<()> where
        T: fmt::Display {
        debug!("ser coll_str");
//        Err(SerializationError::UnserializableType.into())
        Ok(())
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

/// Serialize the given data structure as PM into the IO stream.
#[inline]
pub fn to_buffer<T: ?Sized>(value: &T) -> Result<SerializedBuffer>
    where
        T: Serialize// + PMSized,
{
    let mut ser = Serializer { buff: SerializedBuffer::new(true) };
    value.serialize(&mut ser)?;
    let size = ser.buff.capacity();
//    let size = value.size_hint().expect("unknown object size");
    debug!("serializing with size {}", size);
    let mut ser = Serializer { buff: SerializedBuffer::new_with_size(size) };
    value.serialize(&mut ser)?;
    ser.buff.rewind();
    debug!("buffer = {:?}", ser.buff.buffer);
    Ok(ser.buff)
}

/// Serialize the given data structure as PM into the IO stream.
#[inline]
pub fn to_boxed_buffer<T: ?Sized>(value: &T) -> Result<SerializedBuffer>
    where T: Serialize + Identifiable {
    use sized::INT_SIZE;

    let mut ser = Serializer { buff: SerializedBuffer::new(true) };
    value.serialize(&mut ser)?;
    let size = ser.buff.capacity() + INT_SIZE;
//    let size = value.size_hint().expect("unknown object size");
    debug!("serializing with size {}", size);
    let id = value.type_id();
    let mut buffer = SerializedBuffer::new_with_size(size);
    buffer.write_u32(id);
    let mut ser = Serializer { buff: buffer };
    value.serialize(&mut ser)?;
    ser.buff.rewind();
    debug!("boxed buffer = {:?}", ser.buff.buffer);
    Ok(ser.buff)
}

/// Serialize the given data structure as PM into the IO stream.
#[inline]
pub fn to_buffer_with_padding<T: ?Sized>(value: &T) -> Result<SerializedBuffer>
    where
        T: Serialize// + PMSized,
{
    let mut ser = Serializer { buff: SerializedBuffer::new(true) };
    value.serialize(&mut ser)?;
    let size = ser.buff.capacity();
//    let size = value.size_hint().expect("unknown object size");
    let padding = (4 - size % 4) % 4;
    let final_size = size + padding;
    debug!("serializing with size {}", final_size);
    let mut ser = Serializer { buff: SerializedBuffer::new_with_size(final_size) };
    value.serialize(&mut ser)?;
    for _ in 0..padding {
        ser.buff.write_byte(0);
    }
    ser.buff.rewind();
    debug!("buffer (padded) = {:?}", ser.buff.buffer);
    Ok(ser.buff)
}

///// Serialize the given data structure as PM into the IO stream.
//#[inline]
//pub fn to_writer<W, T: ?Sized>(writer: W, value: &T) -> Result<()>
//    where
//        W: io::Write,
//        T: Serialize,
//{
//    let mut ser = Serializer::new(writer);
//    value.serialize(&mut ser)?;
//    Ok(())
//}
//
///// Serialize the given data structure as a PM byte vector.
//#[inline]
//pub fn to_vec<T: ?Sized>(value: &T) -> Result<Vec<u8>>
//    where
//        T: Serialize,
//{
//    let mut writer = Vec::with_capacity(128);
//    to_writer(&mut writer, value)?;
//    Ok(writer)
//}

///// Serialize the given data structure as a String of PM.
//#[inline]
//pub fn to_serialized_buffer<T: ?Sized>(value: &T) -> Result<SerializedBuffer>
//    where
//        T: Serialize,
//{
//    let vec = to_vec(value)?;
//}
