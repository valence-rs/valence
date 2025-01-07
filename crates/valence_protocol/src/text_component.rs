use std::io::Write;

use anyhow::ensure;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};
use valence_nbt::binary::{FromModifiedUtf8, ToModifiedUtf8};
use valence_nbt::serde::ser::CompoundSerializer;
use valence_nbt::{Compound, Tag};
use valence_text::{IntoText, Text};

use crate::{Decode, Encode};

/// A wrapper around `Text` that encodes and decodes as an NBT Compound.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NbtCompoundText(pub Text);

/// A wrapper around `Text` that encodes and decodes as an NBT String.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NbtStringText(pub Text);

impl Encode for NbtCompoundText {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        self.serialize(CompoundSerializer)?.encode(w)
    }
}

impl Decode<'_> for NbtCompoundText {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        Ok(Self(Text::deserialize(Compound::decode(r)?)?))
    }
}

impl Encode for NbtStringText {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        w.write(&[Tag::String as u8])?;

        let string = self.0.to_legacy_lossy();
        let len = string.modified_uf8_len();

        match len.try_into() {
            Ok(n) => w.write_u16::<BigEndian>(n)?,
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "string of length {len} exceeds maximum of u16::MAX"
                ))
            }
        }

        string.to_modified_utf8(len, &mut w)?;
        Ok(())
    }
}

impl Decode<'_> for NbtStringText {
    fn decode(r: &mut &'_ [u8]) -> anyhow::Result<Self> {
        let tag = r.read_u8()?;
        ensure!(
            tag == Tag::String as u8,
            "tag is not NBT string {}, got {}",
            Tag::String as u8,
            tag
        );

        let len = r.read_u16::<BigEndian>()?.into();
        ensure!(
            len <= r.len(),
            "string of length {} exceeds remainder of input {}",
            len,
            r.len()
        );

        let (left, right) = r.split_at(len);

        let string = match String::from_modified_utf8(left) {
            Ok(string) => {
                *r = right;
                string
            }
            Err(_) => return Err(anyhow::anyhow!("could not decode modified UTF-8 data")),
        };

        Ok(Self(string.into_text()))
    }
}
