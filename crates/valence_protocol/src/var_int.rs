use std::io::{Read, Write};

use anyhow::bail;
use byteorder::ReadBytesExt;
use thiserror::Error;

use crate::{Decode, Encode};

/// An `i32` encoded with variable length.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(transparent)]
pub struct VarInt(pub i32);

impl VarInt {
    /// The maximum number of bytes a VarInt could occupy when read from and
    /// written to the Minecraft protocol.
    pub const MAX_SIZE: usize = 5;

    /// Returns the exact number of bytes this varint will write when
    /// [`Encode::encode`] is called, assuming no error occurs.
    pub fn written_size(self) -> usize {
        match self.0 {
            0 => 1,
            n => (31 - n.leading_zeros() as usize) / 7 + 1,
        }
    }

    pub fn decode_partial(mut r: impl Read) -> Result<i32, VarIntDecodeError> {
        let mut val = 0;
        for i in 0..Self::MAX_SIZE {
            let byte = r.read_u8().map_err(|_| VarIntDecodeError::Incomplete)?;
            val |= (byte as i32 & 0b01111111) << (i * 7);
            if byte & 0b10000000 == 0 {
                return Ok(val);
            }
        }

        Err(VarIntDecodeError::TooLarge)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Error)]
pub enum VarIntDecodeError {
    #[error("incomplete VarInt decode")]
    Incomplete,
    #[error("VarInt is too large")]
    TooLarge,
}

impl Encode for VarInt {
    // Adapted from Moulberry's encode
    // https://github.com/Moulberry/Graphite/blob/master/crates/graphite_binary/src/varint/encode.rs#L6
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        let x = unsafe { std::mem::transmute::<i32, u32>(self.0) } as u64;
        let stage1 = (x & 0x000000000000007f)
            | ((x & 0x0000000000003f80) << 1)
            | ((x & 0x00000000001fc000) << 2)
            | ((x & 0x000000000fe00000) << 3)
            | ((x & 0x00000000f0000000) << 4);

        let leading = stage1.leading_zeros();

        let unused_bytes = (leading - 1) >> 3;
        let bytes_needed = 8 - unused_bytes;

        // set all but the last MSBs
        let msbs = 0x8080808080808080;
        let msbmask = 0xffffffffffffffff >> (((8 - bytes_needed + 1) << 3) - 1);

        let merged = stage1 | (msbs & msbmask);
        let bytes = merged.to_le_bytes();

        w.write_all(&bytes[..bytes_needed as usize])?;

        Ok(())
    }
}

impl Decode<'_> for VarInt {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        let mut val = 0;
        for i in 0..Self::MAX_SIZE {
            let byte = r.read_u8()?;
            val |= (byte as i32 & 0b01111111) << (i * 7);
            if byte & 0b10000000 == 0 {
                return Ok(VarInt(val));
            }
        }
        bail!("VarInt is too large")
    }
}

impl From<i32> for VarInt {
    fn from(i: i32) -> Self {
        VarInt(i)
    }
}

impl From<VarInt> for i32 {
    fn from(i: VarInt) -> Self {
        i.0
    }
}

#[cfg(test)]
mod tests {
    use rand::{thread_rng, Rng};

    use super::*;

    #[test]
    fn varint_written_size() {
        let mut rng = thread_rng();
        let mut buf = vec![];

        for n in (0..100_000)
            .map(|_| rng.gen())
            .chain([0, i32::MIN, i32::MAX])
            .map(VarInt)
        {
            buf.clear();
            n.encode(&mut buf).unwrap();
            assert_eq!(buf.len(), n.written_size());
        }
    }

    #[test]
    fn varint_round_trip() {
        let mut rng = thread_rng();
        let mut buf = vec![];

        for n in (0..1_000_000)
            .map(|_| rng.gen())
            .chain([0, i32::MIN, i32::MAX])
        {
            VarInt(n).encode(&mut buf).unwrap();

            let mut slice = buf.as_slice();
            assert!(slice.len() <= VarInt::MAX_SIZE);

            assert_eq!(n, VarInt::decode(&mut slice).unwrap().0);

            assert!(slice.is_empty());
            buf.clear();
        }
    }
}
