use std::fmt;
use std::io;

use serde::ser::{self, Impossible, Serialize};
use super::error::{Error, Result};
use super::serializable::SerializedBuffer;

struct Serializer {
    buff: SerializedBuffer
}

impl ser::Serializer for Serializer {
    type Ok = ();
    type Error = ();
    type SerializeSeq = ();
    type SerializeTuple = ();
    type SerializeTupleStruct = ();
    type SerializeTupleVariant = ();
    type SerializeMap = ();
    type SerializeStruct = ();
    type SerializeStructVariant = ();

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.buff.write_bool(v)?;
        buff
    }

    fn serialize_i8(self, v: i8) -> Result<()> {
        unimplemented!()
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        unimplemented!()
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        unimplemented!()
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        unimplemented!()
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        unimplemented!()
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        unimplemented!()
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        unimplemented!()
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        unimplemented!()
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        unimplemented!()
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        unimplemented!()
    }

    fn serialize_char(self, v: char) -> Result<()> {
        unimplemented!()
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        unimplemented!()
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        unimplemented!()
    }

    fn serialize_none(self) -> Result<()> {
        unimplemented!()
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<()> where
        T: Serialize {
        unimplemented!()
    }

    fn serialize_unit(self) -> Result<()> {
        unimplemented!()
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<()> {
        unimplemented!()
    }

    fn serialize_unit_variant(self, name: &'static str, variant_index: u32, variant: &'static str) -> Result<()> {
        unimplemented!()
    }

    fn serialize_newtype_struct<T: ?Sized>(self, name: &'static str, value: &T) -> Result<()> where
        T: Serialize {
        unimplemented!()
    }

    fn serialize_newtype_variant<T: ?Sized>(self, name: &'static str, variant_index: u32, variant: &'static str, value: &T) -> Result<()> where
        T: Serialize {
        unimplemented!()
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        unimplemented!()
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        unimplemented!()
    }

    fn serialize_tuple_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeTupleStruct> {
        unimplemented!()
    }

    fn serialize_tuple_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeTupleVariant> {
        unimplemented!()
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        unimplemented!()
    }

    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        unimplemented!()
    }

    fn serialize_struct_variant(self, name: &'static str, variant_index: u32, variant: &'static str, len: usize) -> Result<Self::SerializeStructVariant> {
        unimplemented!()
    }

    fn collect_str<T: ?Sized>(self, value: &T) -> Result<()> where
        T: fmt::Display {
        unimplemented!()
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

/// Serialize the given data structure as PM into the IO stream.
#[inline]
pub fn to_writer<W, T: ?Sized>(writer: W, value: &T) -> Result<()>
    where
        W: io::Write,
        T: Serialize,
{
    let mut ser = Serializer::new(writer);
    value.serialize(&mut ser)?;
    Ok(())
}

/// Serialize the given data structure as a PM byte vector.
#[inline]
pub fn to_vec<T: ?Sized>(value: &T) -> Result<Vec<u8>>
    where
        T: Serialize,
{
    let mut writer = Vec::with_capacity(128);
    to_writer(&mut writer, value)?;
    Ok(writer)
}

/// Serialize the given data structure as a String of PM.
#[inline]
pub fn to_serialized_buffer<T: ?Sized>(value: &T) -> Result<SerializedBuffer>
    where
        T: Serialize,
{
    let vec = to_vec(value)?;
//    let string = unsafe {
//        // We do not emit invalid UTF-8.
//        String::from_utf8_unchecked(vec)
//    };
//    Ok(string)
}
