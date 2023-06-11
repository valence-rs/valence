use std::io::Write;
use std::mem;

use crate::protocol::{Decode, Encode};

/// While [encoding], the contained slice is written directly to the output
/// without any length prefix or metadata.
///
/// While [decoding], the remainder of the input is returned as the contained
/// slice. The input will be at the EOF state after this is finished.
///
/// [encoding]: Encode
/// [decoding]: Decode
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct RawBytes<'a>(pub &'a [u8]);

impl Encode for RawBytes<'_> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        Ok(w.write_all(self.0)?)
    }
}

impl<'a> Decode<'a> for RawBytes<'a> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        Ok(Self(mem::take(r)))
    }
}

impl<'a> From<&'a [u8]> for RawBytes<'a> {
    fn from(value: &'a [u8]) -> Self {
        Self(value)
    }
}

impl<'a> From<RawBytes<'a>> for &'a [u8] {
    fn from(value: RawBytes<'a>) -> Self {
        value.0
    }
}
