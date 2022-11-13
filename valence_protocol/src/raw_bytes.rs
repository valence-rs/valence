use std::io::Write;

use crate::{Decode, Encode, Result};

/// While [encoding], the contained slice is written directly to the output
/// without any metadata.
///
/// While [decoding], the remainder of the input is returned as the contained
/// slice. The input will be at the EOF state after this is finished.
///
/// [encoding]: Encode
/// [decoding]: Decode
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct RawBytes<'a>(pub &'a [u8]);

impl Encode for RawBytes<'_> {
    fn encode(&self, mut w: impl Write) -> Result<()> {
        Ok(w.write_all(self.0)?)
    }

    fn encoded_len(&self) -> usize {
        self.0.len()
    }
}

impl<'a> Decode<'a> for RawBytes<'a> {
    fn decode(r: &mut &'a [u8]) -> Result<Self> {
        let slice = *r;
        *r = &[];
        Ok(Self(slice))
    }
}
