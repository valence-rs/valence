use std::io::{Read, Write};

use anyhow::bail;
use byteorder::{ReadBytesExt, WriteBytesExt};
use thiserror::Error;

use crate::{Decode, Encode};

/// An `i32` encoded with variable length.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct VarInt(pub i32);

impl VarInt {
    /// The maximum number of bytes a VarInt could occupy when read from and
    /// written to the Minecraft protocol.
    pub const MAX_SIZE: usize = 5;

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
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        let mut val = self.0 as u32;
        loop {
            if val & 0b11111111111111111111111110000000 == 0 {
                w.write_u8(val as u8)?;
                return Ok(());
            }
            w.write_u8(val as u8 & 0b01111111 | 0b10000000)?;
            val >>= 7;
        }
    }

    fn encoded_len(&self) -> usize {
        match self.0 {
            0 => 1,
            n => (31 - n.leading_zeros() as usize) / 7 + 1,
        }
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
    fn encoded_len_correct() {
        let mut rng = thread_rng();
        let mut buf = vec![];

        for n in (0..100_000)
            .map(|_| rng.gen())
            .chain([0, i32::MIN, i32::MAX])
            .map(VarInt)
        {
            buf.clear();
            n.encode(&mut buf).unwrap();
            assert_eq!(buf.len(), n.encoded_len());
        }
    }

    #[test]
    fn encode_decode() {
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
