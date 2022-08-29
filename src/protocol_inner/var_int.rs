use std::io::{Read, Write};

use anyhow::bail;
use byteorder::{ReadBytesExt, WriteBytesExt};

use crate::protocol_inner::{Decode, Encode};

/// An `i32` encoded with variable length.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct VarInt(pub i32);

impl VarInt {
    /// The maximum number of bytes a VarInt could occupy when read from and
    /// written to the Minecraft protocol.
    pub const MAX_SIZE: usize = 5;

    /// The number of bytes this `VarInt` will occupy when written to the
    /// Minecraft protocol.
    pub const fn written_size(self) -> usize {
        let val = self.0 as u32;
        if val & 0b11110000_00000000_00000000_00000000 != 0 {
            5
        } else if val & 0b11111111_11100000_00000000_00000000 != 0 {
            4
        } else if val & 0b11111111_11111111_11000000_00000000 != 0 {
            3
        } else if val & 0b11111111_11111111_11111111_10000000 != 0 {
            2
        } else {
            1
        }
    }
}

impl Encode for VarInt {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
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
}

impl Decode for VarInt {
    fn decode(r: &mut impl Read) -> anyhow::Result<Self> {
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

impl From<VarInt> for i32 {
    fn from(i: VarInt) -> Self {
        i.0
    }
}

impl From<VarInt> for i64 {
    fn from(i: VarInt) -> Self {
        i.0 as i64
    }
}

impl From<i32> for VarInt {
    fn from(i: i32) -> Self {
        VarInt(i)
    }
}

#[cfg(test)]
mod tests {
    use rand::{thread_rng, Rng};

    use super::*;

    #[test]
    fn written_size_correct() {
        let mut rng = thread_rng();
        let mut buf = Vec::new();

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
    fn encode_decode() {
        let mut rng = thread_rng();
        let mut buf = Vec::new();

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
