use std::io::Write;
use std::str::FromStr;

use anyhow::{ensure, Context};
use valence_text::Text;

use crate::{Bounded, Decode, Encode, VarInt};

const DEFAULT_MAX_STRING_CHARS: usize = 32767;
const MAX_TEXT_CHARS: usize = 262144;

impl Encode for str {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        Bounded::<_, DEFAULT_MAX_STRING_CHARS>(self).encode(w)
    }
}

impl<const MAX_CHARS: usize> Encode for Bounded<&'_ str, MAX_CHARS> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        let char_count = self.chars().count();

        ensure!(
            char_count <= MAX_CHARS,
            "char count of string exceeds maximum (expected <= {MAX_CHARS}, got {char_count})"
        );

        VarInt(self.len() as i32).encode(&mut w)?;
        Ok(w.write_all(self.as_bytes())?)
    }
}

impl<'a> Decode<'a> for &'a str {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        Ok(Bounded::<_, DEFAULT_MAX_STRING_CHARS>::decode(r)?.0)
    }
}

impl<'a, const MAX_CHARS: usize> Decode<'a> for Bounded<&'a str, MAX_CHARS> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let len = VarInt::decode(r)?.0;
        ensure!(len >= 0, "attempt to decode string with negative length");
        let len = len as usize;
        ensure!(
            len < r.len(),
            "not enough data remaining to decode string of {len} bytes"
        );

        let (res, remaining) = r.split_at(len);
        let res = std::str::from_utf8(res)?;

        let char_count = res.chars().count();
        ensure!(
            char_count <= MAX_CHARS,
            "char count of string exceeds maximum (expected <= {MAX_CHARS}, got {char_count})"
        );

        *r = remaining;

        Ok(Bounded(res))
    }
}

impl Encode for String {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        self.as_str().encode(w)
    }
}

impl<const MAX_CHARS: usize> Encode for Bounded<String, MAX_CHARS> {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        Bounded::<_, MAX_CHARS>(self.as_str()).encode(w)
    }
}

impl Decode<'_> for String {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        Ok(<&str>::decode(r)?.into())
    }
}

impl<const MAX_CHARS: usize> Decode<'_> for Bounded<String, MAX_CHARS> {
    fn decode(r: &mut &'_ [u8]) -> anyhow::Result<Self> {
        Ok(Bounded(Bounded::<&str, MAX_CHARS>::decode(r)?.0.into()))
    }
}

impl Decode<'_> for Box<str> {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        Ok(<&str>::decode(r)?.into())
    }
}

impl<const MAX_CHARS: usize> Decode<'_> for Bounded<Box<str>, MAX_CHARS> {
    fn decode(r: &mut &'_ [u8]) -> anyhow::Result<Self> {
        Ok(Bounded(Bounded::<&str, MAX_CHARS>::decode(r)?.0.into()))
    }
}

impl Encode for Text {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        let s = serde_json::to_string(self).context("serializing text JSON")?;

        Bounded::<_, MAX_TEXT_CHARS>(s).encode(w)
    }
}

impl Decode<'_> for Text {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        let str = Bounded::<&str, MAX_TEXT_CHARS>::decode(r)?.0;

        Self::from_str(str).context("deserializing text JSON")
    }
}
