use std::io::Read;

use anyhow::anyhow;
use byteorder::ReadBytesExt;
use serde::de;
use serde::de::DeserializeSeed;

use crate::binary::de::payload::PayloadDeserializer;
use crate::{Error, Tag};

pub struct MapAccess<'r, R: ?Sized> {
    reader: &'r mut R,
    value_tag: Tag,
    /// Provides error context when deserializing structs.
    fields: &'static [&'static str],
}

impl<'r, R: Read + ?Sized> MapAccess<'r, R> {
    pub fn new(reader: &'r mut R, fields: &'static [&'static str]) -> Self {
        Self {
            reader,
            value_tag: Tag::End,
            fields,
        }
    }
}

impl<'de: 'r, 'r, R: Read + ?Sized> de::MapAccess<'de> for MapAccess<'r, R> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
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

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
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
