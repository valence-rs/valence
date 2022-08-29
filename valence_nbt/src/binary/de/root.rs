use std::borrow::Cow;
use std::io::Read;

use anyhow::anyhow;
use byteorder::{BigEndian, ReadBytesExt};
use cesu8::from_java_cesu8;
use serde::de::Visitor;
use serde::{forward_to_deserialize_any, Deserializer};
use smallvec::SmallVec;

use crate::binary::de::payload::PayloadDeserializer;
use crate::{Error, Tag};

#[non_exhaustive]
pub struct RootDeserializer<R> {
    pub reader: R,
    pub root_name: String,
    pub save_root_name: bool,
}

impl<R: Read> RootDeserializer<R> {
    pub fn new(reader: R, save_root_name: bool) -> Self {
        Self {
            reader,
            root_name: String::new(),
            save_root_name,
        }
    }

    fn read_name(&mut self) -> Result<Tag, Error> {
        let tag = Tag::from_u8(self.reader.read_u8()?)?;

        if tag != Tag::Compound {
            return Err(Error(anyhow!(
                "unexpected tag `{tag}` (root value must be a compound)"
            )));
        }

        if self.save_root_name {
            let mut buf = SmallVec::<[u8; 128]>::new();
            for _ in 0..self.reader.read_u16::<BigEndian>()? {
                buf.push(self.reader.read_u8()?);
            }

            match from_java_cesu8(&buf).map_err(|e| Error(anyhow!(e)))? {
                Cow::Borrowed(s) => s.clone_into(&mut self.root_name),
                Cow::Owned(s) => self.root_name = s,
            }
        } else {
            for _ in 0..self.reader.read_u16::<BigEndian>()? {
                self.reader.read_u8()?;
            }
        }

        Ok(tag)
    }
}

impl<'de: 'a, 'a, R: Read> Deserializer<'de> for &'a mut RootDeserializer<R> {
    type Error = Error;

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map enum identifier ignored_any
    }

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tag = self.read_name()?;

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
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let tag = self.read_name()?;

        PayloadDeserializer {
            reader: &mut self.reader,
            tag,
        }
        .deserialize_struct(name, fields, visitor)
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}
