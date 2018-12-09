use std::fmt;
use std::io;

use serde::de::{self, Deserialize};
use super::error::{Error, Result, SerializationError};
use super::serializable::SerializedBuffer;
use SVUID;

use utils::{safe_uint_cast};

pub struct Deserializer<'ids> {
    buff: &'ids mut SerializedBuffer,
    enum_variant_ids: &'ids [&'static str]
}

impl<'ids> Deserializer<'ids> {
    pub fn new(stream: &'ids mut SerializedBuffer, enum_variant_ids: &'ids [&'static str]) -> Self {
        Deserializer {
            buff: stream,
            enum_variant_ids
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
        visitor.visit_bool(self.buff.read_bool()?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_i8");
        visitor.visit_i8(self.buff.read_i8()?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_i16");
        Err(SerializationError::UnserializableType.into())
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_i32");
        visitor.visit_i32(self.buff.read_i32()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_i64");
        visitor.visit_i64(self.buff.read_i64()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_u8");
        visitor.visit_u8(self.buff.read_u8()?)

    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_u16");
        Err(SerializationError::UnserializableType.into())
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_u32");
        visitor.visit_u32(self.buff.read_u32()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_u64");
        visitor.visit_u64(self.buff.read_u64()?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_f32");
        visitor.visit_f32(self.buff.read_f32()?)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_f64");
        visitor.visit_f64(self.buff.read_f64()?)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_char");
        visitor.visit_char(self.buff.read_byte()? as char)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_str");
        visitor.visit_str(&self.buff.read_string()?)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_string");
        visitor.visit_string(self.buff.read_string()?)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_bytes");
        visitor.visit_bytes(&self.buff.read_byte_array()?)
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_byte_buf");
        visitor.visit_byte_buf(self.buff.read_byte_array()?)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_option");
        Err(SerializationError::UnserializableType.into())
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_unit");
        Err(SerializationError::UnserializableType.into())
    }

    fn deserialize_unit_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_unit_struct");
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_newtype_struct");
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_seq...");
        let len = self.buff.read_u32()?;
        debug!("  ...with len {}", len);
        visitor.visit_seq(SeqAccess::new(self, len))
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_tuple");
        visitor.visit_seq(SeqAccess::new(self, safe_uint_cast(len)?))
    }

    fn deserialize_tuple_struct<V>(self, name: &'static str, len: usize, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_tuple_struct");
        visitor.visit_seq(SeqAccess::new(self, safe_uint_cast(len)?))
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_map");
        Err(SerializationError::UnserializableType.into())
    }

    fn deserialize_struct<V>(self, name: &'static str, fields: &'static [&'static str], visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_struct {}", name);
        for f in fields {
            debug!("> field {}", *f);
        }
        visitor.visit_seq(SeqAccess::new(self, safe_uint_cast(fields.len())?))
    }

    fn deserialize_enum<V>(self, name: &'static str, variants: &'static [&'static str], visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_enum name={} vars={:?}", name, variants);
        visitor.visit_enum(EnumVariantAccess::new(self))
//        visitor.visit_enum(EnumVariantAccess::new(self))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_identifier");
        // TODO: make right error type
        let variant_id = self.buff.read_u8()?;
//
//        let (variant_id, rest) = self.enum_variant_ids.split_first()
//            .ok_or_else(|| SerializationError::UnserializableType)?;
//
//        debug!("Deserialized variant_id {}", variant_id);
//        self.enum_variant_ids = rest;

//        V::Value::get
        visitor.visit_u32(variant_id as u32)
//        visitor.visit_str(variant_id)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value> where
        V: de::Visitor<'de> {
        debug!("deserialize_ignored_any");
        Err(SerializationError::UnserializableType.into())
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

struct EnumVariantAccess<'a, 'ids: 'a> {
    de: &'a mut Deserializer<'ids>,
}

impl<'a, 'ids> EnumVariantAccess<'a, 'ids> {
    fn new(de: &'a mut Deserializer<'ids>) -> EnumVariantAccess<'a, 'ids> {
        EnumVariantAccess { de }
    }
}

impl<'de, 'a, 'ids> de::EnumAccess<'de> for EnumVariantAccess<'a, 'ids>
{
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
        where V: de::DeserializeSeed<'de>
    {
        use serde::de::IntoDeserializer;
        
        debug!("Deserializing enum variant");

        let index: u8 = Deserialize::deserialize(&mut *self.de)?;
        let value: Result<_> = seed.deserialize(index.into_deserializer());

        Ok((value?, self))
    }
}

impl<'de, 'a, 'ids> de::VariantAccess<'de> for EnumVariantAccess<'a, 'ids>
{
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        debug!("Deserialized unit variant");
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
        where T: de::DeserializeSeed<'de>
    {
        debug!("Deserializing newtype variant");
        seed.deserialize(self.de)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
        where V: de::Visitor<'de>
    {
        debug!("Deserializing tuple variant");
        de::Deserializer::deserialize_tuple_struct(self.de, "", len, visitor)
    }

    fn struct_variant<V>(self, fields: &'static [&'static str], visitor: V) -> Result<V::Value>
        where V: de::Visitor<'de>
    {
        debug!("Deserializing struct variant");
        de::Deserializer::deserialize_struct(self.de, "", fields, visitor)
    }
}

pub fn from_stream<'de, T>(stream: &'de mut SerializedBuffer, enum_variant_ids: &[&'static str]) -> Result<T>
    where
        T: de::Deserialize<'de>,
{
    let mut de = Deserializer::new(stream, enum_variant_ids);
    let value = T::deserialize(&mut de)?;

    Ok(value)
}