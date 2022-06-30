use std::io::{Read, Write};

use anyhow::bail;
use byteorder::{ReadBytesExt, WriteBytesExt};

use crate::protocol::{Decode, Encode};

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct VarLong(pub(crate) i64);

impl VarLong {
    /// The maximum number of bytes a `VarLong` can occupy when read from and
    /// written to the Minecraft protocol.
    pub const MAX_SIZE: usize = 10;
}

impl Encode for VarLong {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        let mut val = self.0 as u64;
        loop {
            if val & 0b1111111111111111111111111111111111111111111111111111111110000000 == 0 {
                w.write_u8(val as u8)?;
                return Ok(());
            }
            w.write_u8(val as u8 & 0b01111111 | 0b10000000)?;
            val >>= 7;
        }
    }
}

impl Decode for VarLong {
    fn decode(r: &mut impl Read) -> anyhow::Result<Self> {
        let mut val = 0;
        for i in 0..Self::MAX_SIZE {
            let byte = r.read_u8()?;
            val |= (byte as i64 & 0b01111111) << (i * 7);
            if byte & 0b10000000 == 0 {
                return Ok(VarLong(val));
            }
        }
        bail!("VarInt is too large")
    }
}

#[cfg(test)]
mod tests {
    use rand::{thread_rng, Rng};

    use super::*;

    #[test]
    fn encode_decode() {
        let mut rng = thread_rng();
        let mut buf = Vec::new();

        for n in (0..1_000_000)
            .map(|_| rng.gen())
            .chain([0, i64::MIN, i64::MAX])
        {
            VarLong(n).encode(&mut buf).unwrap();

            let mut slice = buf.as_slice();
            assert!(slice.len() <= VarLong::MAX_SIZE);

            assert_eq!(n, VarLong::decode(&mut slice).unwrap().0);
            assert!(slice.is_empty());
            buf.clear();
        }
    }
}
