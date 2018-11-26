use std::fmt;
use std::io;

use serde::ser::{self, Impossible, Serialize};
use super::error::{Error, Result, SerializationError};
use super::serializable::SerializedBuffer;
use SVUID;

struct Serializer {
    buff: SerializedBuffer
}

struct SerSeq/*<S: Serialize>*/ {
//    ser: S
}
struct SerStruct<'a> {
    ser: &'a mut Serializer,
}
struct SerNone {}

impl ser::SerializeTuple for SerNone {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()> where
        T: Serialize {
//        value.serialize(self)
        Ok(())
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl ser::SerializeTupleStruct for SerNone {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()> where
        T: Serialize {
//        value.serialize(self)
        Ok(())
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl ser::SerializeTupleVariant for SerNone {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()> where
        T: Serialize {
//        value.serialize(self)
        Ok(())
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

impl ser::SerializeStructVariant for SerNone {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()> where
        T: Serialize {
        Err(SerializationError::UnserializableType.into())
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl ser::SerializeSeq for SerSeq {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()> where
        T: Serialize {
        debug!("> ser element");
//        value.serialize(self.ser)
        Ok(())
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
        value.serialize(&mut *self.ser)
//        Ok(())
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = SerSeq;
    type SerializeTuple = SerNone;
    type SerializeTupleStruct = SerNone;
    type SerializeTupleVariant = SerNone;
    type SerializeMap = SerNone;
    type SerializeStruct = SerStruct<'a>;
    type SerializeStructVariant = SerNone;

    fn serialize_bool(self, v: bool) -> Result<()> {
        debug!("serialize_bool");
        self.buff.write_bool(v)
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        debug!("serialize_i8");
        self.buff.write_i8(v)
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        debug!("serialize_i16");
        Err(SerializationError::UnserializableType.into())
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        debug!("serialize_i32");
        self.buff.write_i32(v)
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        debug!("serialize_i64");
        self.buff.write_i64(v)
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.buff.write_u8(v)
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        debug!("serialize_u16");
        Err(SerializationError::UnserializableType.into())
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.buff.write_u32(v)
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.buff.write_u64(v)
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        self.buff.write_f32(v)
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        self.buff.write_f64(v)
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.buff.write_u32(v as u32)
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.buff.write_string(v)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        debug!("ser bytes {}", v.len());
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
        self.buff.write_u32(variant_index)
//        Ok(())
    }

    fn serialize_newtype_struct<T: ?Sized>(self, name: &'static str, value: &T) -> Result<()> where
        T: Serialize {
        debug!("ser struct {}", name);
//        self.buff.write_u32(T::svuid());
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(self, name: &'static str, variant_index: u32, variant: &'static str, value: &T) -> Result<()> where
        T: Serialize {
        debug!("serialize_newtype_variant");

//        Err(SerializationError::UnserializableType.into())
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        debug!("ser seq {:?}", len);

        match len {
            Some(len) => {
                self.buff.write_u32(len as u32)?;
                Ok(SerSeq { /*ser: self*/ })
            },
            None => Err(SerializationError::UnserializableType.into())
        }
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        debug!("serialize_tuple");
        Err(SerializationError::UnserializableType.into())
    }

    fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeTupleStruct> {
        debug!("serialize_tuple_struct");
        Err(SerializationError::UnserializableType.into())
    }

    fn serialize_tuple_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeTupleVariant> {
        debug!("serialize_tuple_variant");
        Err(SerializationError::UnserializableType.into())
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        debug!("serialize_map");
        Err(SerializationError::UnserializableType.into())
    }

    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        debug!("serialize_struct");
        Ok(SerStruct { ser: self })
//        Err(SerializationError::UnserializableType.into())
    }

    fn serialize_struct_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeStructVariant> {
        debug!("serialize_struct_variant");
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
        T: Serialize,
{
    let mut ser_calc = Serializer { buff: SerializedBuffer::new(true) };
    value.serialize(&mut ser_calc)?;
    let size = ser_calc.buff.capacity();
    debug!("serializing with size {}", size);
    let mut ser = Serializer { buff: SerializedBuffer::new_with_size(size) };
    value.serialize(&mut ser)?;
    ser.buff.rewind();
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
