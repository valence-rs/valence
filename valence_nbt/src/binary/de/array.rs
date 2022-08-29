use std::io::Read;

use byteorder::{BigEndian, ReadBytesExt};
use serde::de::value::StrDeserializer;
use serde::de::{DeserializeSeed, Error as _, SeqAccess, Unexpected, Visitor};
use serde::{de, forward_to_deserialize_any, Deserializer};

use crate::binary::de::payload::PayloadDeserializer;
use crate::{
    ArrayType, Error, BYTE_ARRAY_VARIANT_NAME, INT_ARRAY_VARIANT_NAME, LONG_ARRAY_VARIANT_NAME,
};

pub struct EnumAccess<'r, R: ?Sized> {
    pub(super) reader: &'r mut R,
    pub(super) array_type: ArrayType,
}

impl<'de: 'r, 'r, R: Read + ?Sized> de::EnumAccess<'de> for EnumAccess<'r, R> {
    type Error = Error;
    type Variant = VariantAccess<'r, R>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let variant_name = match self.array_type {
            ArrayType::Byte => BYTE_ARRAY_VARIANT_NAME,
            ArrayType::Int => INT_ARRAY_VARIANT_NAME,
            ArrayType::Long => LONG_ARRAY_VARIANT_NAME,
        };

        Ok((
            seed.deserialize(StrDeserializer::<Error>::new(variant_name))?,
            VariantAccess {
                reader: self.reader,
                array_type: self.array_type,
            },
        ))
    }
}

pub struct VariantAccess<'r, R: ?Sized> {
    reader: &'r mut R,
    array_type: ArrayType,
}

impl<'de: 'r, 'r, R: Read + ?Sized> de::VariantAccess<'de> for VariantAccess<'r, R> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Err(Error::invalid_type(
            Unexpected::NewtypeVariant,
            &"unit variant",
        ))
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(ArrayDeserializer {
            reader: self.reader,
            array_type: self.array_type,
        })
    }

    fn tuple_variant<V>(self, _len: usize, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::invalid_type(
            Unexpected::NewtypeVariant,
            &"tuple variant",
        ))
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(Error::invalid_type(
            Unexpected::NewtypeVariant,
            &"struct variant",
        ))
    }
}

struct ArrayDeserializer<'r, R: ?Sized> {
    reader: &'r mut R,
    array_type: ArrayType,
}

impl<'de: 'r, 'r, R: Read + ?Sized> Deserializer<'de> for ArrayDeserializer<'r, R> {
    type Error = Error;

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let len = self.reader.read_i32::<BigEndian>()?;

        if len < 0 {
            return Err(Error::new_static("array with negative length"));
        }

        visitor.visit_seq(ArraySeqAccess {
            reader: self.reader,
            array_type: self.array_type,
            remaining: len,
        })
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

struct ArraySeqAccess<'r, R: ?Sized> {
    reader: &'r mut R,
    array_type: ArrayType,
    remaining: i32,
}

impl<'de: 'r, 'r, R: Read + ?Sized> SeqAccess<'de> for ArraySeqAccess<'r, R> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        if self.remaining > 0 {
            self.remaining -= 1;

            seed.deserialize(PayloadDeserializer {
                reader: self.reader,
                tag: self.array_type.element_tag(),
            })
            .map(Some)
        } else {
            Ok(None)
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.remaining as usize)
    }
}
