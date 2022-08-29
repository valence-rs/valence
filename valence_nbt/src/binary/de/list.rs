use std::io::Read;

use serde::de;
use serde::de::DeserializeSeed;

use crate::binary::de::payload::PayloadDeserializer;
use crate::{Error, Tag};

pub(super) struct SeqAccess<'r, R: ?Sized> {
    pub reader: &'r mut R,
    pub element_tag: Tag,
    pub remaining: u32,
}

impl<'de: 'r, 'r, R: Read + ?Sized> de::SeqAccess<'de> for SeqAccess<'r, R> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: DeserializeSeed<'de>,
    {
        if self.remaining > 0 {
            self.remaining -= 1;

            seed.deserialize(PayloadDeserializer {
                reader: self.reader,
                tag: self.element_tag,
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
