// TODO: recursion limit.
// TODO: serialize and deserialize recursion limit wrappers. (crate:
// serde_reclimit).

use std::borrow::Cow;
use std::io::Read;

use anyhow::anyhow;
use byteorder::{BigEndian, ReadBytesExt};
use cesu8::from_java_cesu8;
use serde::de::{DeserializeOwned, DeserializeSeed, Visitor};
use serde::{de, forward_to_deserialize_any};
use smallvec::SmallVec;

use crate::{Error, Result, Tag};

pub fn from_reader<R, T>(reader: R) -> Result<T>
where
    R: Read,
    T: DeserializeOwned,
{
    T::deserialize(&mut Deserializer::new(reader, false))
}

pub struct Deserializer<R> {
    reader: R,
    root_name: Option<String>,
}

impl<R: Read> Deserializer<R> {
    pub fn new(reader: R, save_root_name: bool) -> Self {
        Self {
            reader,
            root_name: if save_root_name {
                Some(String::new())
            } else {
                None
            },
        }
    }

    pub fn into_inner(self) -> (R, Option<String>) {
        (self.reader, self.root_name)
    }

    fn read_header(&mut self) -> Result<Tag> {
        let tag = Tag::from_u8(self.reader.read_u8()?)?;

        if tag != Tag::Compound {
            return Err(Error(anyhow!(
                "unexpected tag `{tag}` (root value must be a compound)"
            )));
        }

        if let Some(name) = &mut self.root_name {
            let mut buf = SmallVec::<[u8; 128]>::new();
            for _ in 0..self.reader.read_u16::<BigEndian>()? {
                buf.push(self.reader.read_u8()?);
            }

            *name = from_java_cesu8(&buf)
                .map_err(|e| Error(anyhow!(e)))?
                .into_owned();
        } else {
            for _ in 0..self.reader.read_u16::<BigEndian>()? {
                self.reader.read_u8()?;
            }
        }

        Ok(tag)
    }
}

impl<'de: 'a, 'a, R: Read + 'de> de::Deserializer<'de> for &'a mut Deserializer<R> {
    type Error = Error;

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct seq tuple tuple_struct map
        enum identifier ignored_any
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let tag = self.read_header()?;

        PayloadDeserializer {
            reader: &mut self.reader,
            tag,
        }
        .deserialize_any(visitor)
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let tag = self.read_header()?;

        PayloadDeserializer {
            reader: &mut self.reader,
            tag,
        }
        .deserialize_struct(name, fields, visitor)
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

struct PayloadDeserializer<'a, R> {
    reader: &'a mut R,
    /// The type of payload to be deserialized.
    tag: Tag,
}

impl<'de: 'a, 'a, R: Read> de::Deserializer<'de> for PayloadDeserializer<'a, R> {
    type Error = Error;

    forward_to_deserialize_any! {
        i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option seq tuple tuple_struct map enum identifier
        ignored_any
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        match self.tag {
            Tag::End => unreachable!("invalid payload tag"),
            Tag::Byte => visitor.visit_i8(self.reader.read_i8()?),
            Tag::Short => visitor.visit_i16(self.reader.read_i16::<BigEndian>()?),
            Tag::Int => visitor.visit_i32(self.reader.read_i32::<BigEndian>()?),
            Tag::Long => visitor.visit_i64(self.reader.read_i64::<BigEndian>()?),
            Tag::Float => visitor.visit_f32(self.reader.read_f32::<BigEndian>()?),
            Tag::Double => visitor.visit_f64(self.reader.read_f64::<BigEndian>()?),
            Tag::ByteArray => {
                let len = self.reader.read_i32::<BigEndian>()?;
                visitor.visit_seq(SeqAccess::new(self.reader, Tag::Byte, len)?)
            }
            Tag::String => {
                let mut buf = SmallVec::<[u8; 128]>::new();
                for _ in 0..self.reader.read_u16::<BigEndian>()? {
                    buf.push(self.reader.read_u8()?);
                }

                match from_java_cesu8(&buf).map_err(|e| Error(anyhow!(e)))? {
                    Cow::Borrowed(s) => visitor.visit_str(s),
                    Cow::Owned(string) => visitor.visit_string(string),
                }
            }
            Tag::List => {
                let element_type = Tag::from_u8(self.reader.read_u8()?)?;
                let len = self.reader.read_i32::<BigEndian>()?;
                visitor.visit_seq(SeqAccess::new(self.reader, element_type, len)?)
            }
            Tag::Compound => visitor.visit_map(MapAccess::new(self.reader, &[])),
            Tag::IntArray => {
                let len = self.reader.read_i32::<BigEndian>()?;
                visitor.visit_seq(SeqAccess::new(self.reader, Tag::Int, len)?)
            }
            Tag::LongArray => {
                let len = self.reader.read_i32::<BigEndian>()?;
                visitor.visit_seq(SeqAccess::new(self.reader, Tag::Long, len)?)
            }
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.tag == Tag::Byte {
            match self.reader.read_i8()? {
                0 => visitor.visit_bool(false),
                1 => visitor.visit_bool(true),
                n => visitor.visit_i8(n),
            }
        } else {
            self.deserialize_any(visitor)
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.tag == Tag::Compound {
            visitor.visit_map(MapAccess::new(self.reader, fields))
        } else {
            self.deserialize_any(visitor)
        }
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

#[doc(hidden)]
pub struct SeqAccess<'a, R> {
    reader: &'a mut R,
    element_type: Tag,
    remaining: u32,
}

impl<'a, R: Read> SeqAccess<'a, R> {
    fn new(reader: &'a mut R, element_type: Tag, len: i32) -> Result<Self> {
        if len < 0 {
            return Err(Error(anyhow!("list with negative length")));
        }

        if element_type == Tag::End && len != 0 {
            return Err(Error(anyhow!(
                "list with TAG_End element type must have length zero"
            )));
        }

        Ok(Self {
            reader,
            element_type,
            remaining: len as u32,
        })
    }

    // TODO: function to check if this is for an array or list.
}

impl<'de: 'a, 'a, R: Read> de::SeqAccess<'de> for SeqAccess<'a, R> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if self.remaining > 0 {
            self.remaining -= 1;

            seed.deserialize(PayloadDeserializer {
                reader: self.reader,
                tag: self.element_type,
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

#[doc(hidden)]
pub struct MapAccess<'a, R> {
    reader: &'a mut R,
    value_tag: Tag,
    /// Provides error context when deserializing structs.
    fields: &'static [&'static str],
}

impl<'a, R: Read> MapAccess<'a, R> {
    fn new(reader: &'a mut R, fields: &'static [&'static str]) -> Self {
        Self {
            reader,
            value_tag: Tag::End,
            fields,
        }
    }
}

impl<'de: 'a, 'a, R: Read> de::MapAccess<'de> for MapAccess<'a, R> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        self.value_tag = Tag::from_u8(self.reader.read_u8()?)?;

        if self.value_tag == Tag::End {
            return Ok(None);
        }

        seed.deserialize(PayloadDeserializer {
            reader: self.reader,
            tag: Tag::String,
        })
        .map(Some)
        .map_err(|e| match self.fields {
            [f, ..] => e.context(anyhow!("compound key (field `{f}`)")),
            [] => e,
        })
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        if self.value_tag == Tag::End {
            return Err(Error(anyhow!("end of compound?")));
        }

        let field = match self.fields {
            [field, rest @ ..] => {
                self.fields = rest;
                Some(*field)
            }
            [] => None,
        };

        seed.deserialize(PayloadDeserializer {
            reader: self.reader,
            tag: self.value_tag,
        })
        .map_err(|e| match field {
            Some(f) => e.context(anyhow!("compound value (field `{f}`)")),
            None => e,
        })
    }
}
