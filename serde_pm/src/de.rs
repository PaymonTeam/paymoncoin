use std::fmt;
use std::io;

use serde::de::{self, Deserialize};
use super::error::{Error, Result, SerializationError};
use super::serializable::SerializedBuffer;

use utils::{safe_uint_cast};

pub struct Deserializer<'de> {
    buff: &'de mut SerializedBuffer,
}

impl<'de> Deserializer<'de> {
    pub fn new(stream: &'de mut SerializedBuffer) -> Self {
        Deserializer {
            buff: stream,
        }
    }
}

//#[derive(Debug)]
struct SeqAccess<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
    len: u32,
    next_index: u32,
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
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

impl<'a, 'de> SeqAccess<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>, len: u32) -> SeqAccess<'a, 'de> {
        SeqAccess { de, len, next_index: 0 }
    }
}

impl<'de, 'a> de::SeqAccess<'de> for SeqAccess<'a, 'de> {
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

struct EnumVariantAccess<'a, 'de: 'a> {
    de: &'a mut Deserializer<'de>,
}

impl<'a, 'de> EnumVariantAccess<'a, 'de> {
    fn new(de: &'a mut Deserializer<'de>) -> EnumVariantAccess<'a, 'de> {
        EnumVariantAccess { de }
    }
}

impl<'de, 'a> de::EnumAccess<'de> for EnumVariantAccess<'a, 'de>
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

impl<'de, 'a> de::VariantAccess<'de> for EnumVariantAccess<'a, 'de>
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

pub fn from_stream<'de, T>(stream: &'de mut SerializedBuffer) -> Result<T>
    where
        T: de::Deserialize<'de>,
{
    let mut de = Deserializer::new(stream);
    let value = T::deserialize(&mut de)?;

    Ok(value)
}