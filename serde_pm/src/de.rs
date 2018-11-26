use std::fmt;
use std::io;

use serde::de::{self, Deserialize};
use super::error::{Error, Result, SerializationError};
use super::serializable::SerializedBuffer;
use SVUID;

struct Deserializer<'ids> {
    buff: &'ids mut SerializedBuffer
}

impl<'ids> Deserializer<'ids> {
    pub fn new(stream: &'ids mut SerializedBuffer) -> Self {
        Deserializer {
            buff: stream
        }
    }
}

impl<'de, 'ids, 'a> de::Deserializer<'de> for &'a mut Deserializer<'ids> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
//        visitor.visiti
        debug!("deserialize_any");
//        Ok(())
        Err(Error::from(SerializationError::BoolError))
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_bool");
        visitor.visit_bool(false)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_i8");
        visitor.visit_i8(0)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_i16");
        visitor.visit_i8(0)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_i32");
        visitor.visit_i32(self.buff.read_i32()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_i64");
        visitor.visit_bool(false)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_u8");
        visitor.visit_bool(false)

    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_u16");
        visitor.visit_bool(false)

    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_u32");
        visitor.visit_bool(false)

    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_u64");
        visitor.visit_bool(false)

    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_f32");
        visitor.visit_bool(false)

    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_f64");
        visitor.visit_bool(false)

    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_char");
        visitor.visit_bool(false)

    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_str");
        visitor.visit_bool(false)

    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_string");
        visitor.visit_bool(false)

    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_bytes");
        visitor.visit_bool(false)

    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_byte_buf");
        visitor.visit_bool(false)

    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_option");
        visitor.visit_bool(false)

    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_unit");
        visitor.visit_bool(false)

    }

    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_unit_struct");
        visitor.visit_bool(false)

    }

    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_newtype_struct");
        visitor.visit_bool(false)

    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_seq");
        visitor.visit_bool(false)

    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_tuple");
        visitor.visit_bool(false)

    }

    fn deserialize_tuple_struct<V>(self, name: &'static str, len: usize, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_tuple_struct");
        visitor.visit_bool(false)

    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_map");
        visitor.visit_bool(false)

    }

    fn deserialize_struct<V>(self, name: &'static str, fields: &'static [&'static str], visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_struct {}", name);
        for f in fields {
            debug!("> field {}", *f);
        }
        // TODO: make safe cast
        visitor.visit_seq(SeqAccess::new(self, fields.len() as u32))
    }

    fn deserialize_enum<V>(self, name: &'static str, variants: &'static [&'static str], visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_enum");
        visitor.visit_bool(false)

    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_identifier");
        visitor.visit_bool(false)

    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_ignored_any");
        visitor.visit_bool(false)

    }
}

//#[derive(Debug)]
struct SeqAccess<'a, 'ids: 'a> {
    de: &'a mut Deserializer<'ids>,
    len: u32,
    next_index: u32,
}

impl<'a, 'ids> SeqAccess<'a, 'ids> {
    fn new(de: &'a mut Deserializer<'ids>, len: u32) -> SeqAccess<'a, 'ids> {
        SeqAccess { de, len, next_index: 0 }
    }
}

impl<'de, 'a, 'ids> de::SeqAccess<'de> for SeqAccess<'a, 'ids> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
        where T: de::DeserializeSeed<'de>
    {
        if self.next_index < self.len {
            self.next_index += 1;
        } else {
            debug!("SeqAccess::next_element_seed() is called when no elements is left to deserialize");
            return Ok(None);
        }

        debug!("Deserializing sequence element");
        seed.deserialize(&mut *self.de).map(Some)
    }

    fn size_hint(&self) -> Option<usize> {
        //TODO: make safe cast
        Some((self.len - self.next_index) as usize)
    }
}

pub fn from_stream<'de, T>(stream: &'de mut SerializedBuffer) -> Result<T>
    where
        T: de::Deserialize<'de>,
{
    let mut de = Deserializer::new(stream);
    let value = T::deserialize(&mut de)?;

    Ok(value)
}