use std::io::{Read, Write};

use anyhow::bail;

use crate::protocol::{Decode, Encode};

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, Debug)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

impl Encode for BlockPos {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        match (self.x, self.y, self.z) {
            (-0x2000000..=0x1ffffff, -0x800..=0x7ff, -0x2000000..=0x1ffffff) => {
                let (x, y, z) = (self.x as u64, self.y as u64, self.z as u64);
                (x << 38 | z << 38 >> 26 | y & 0xfff).encode(w)
            }
            _ => bail!(
                "block position {:?} is out of range",
                (self.x, self.y, self.z)
            ),
        }
    }
}

impl Decode for BlockPos {
    fn decode(r: &mut impl Read) -> anyhow::Result<Self> {
        // Use arithmetic right shift to determine sign.
        let val = i64::decode(r)?;
        let x = val >> 38;
        let z = val << 26 >> 38;
        let y = val << 52 >> 52;
        Ok(Self {
            x: x as i32,
            y: y as i32,
            z: z as i32,
        })
    }
}

impl From<(i32, i32, i32)> for BlockPos {
    fn from((x, y, z): (i32, i32, i32)) -> Self {
        BlockPos::new(x, y, z)
    }
}

impl From<BlockPos> for (i32, i32, i32) {
    fn from(pos: BlockPos) -> Self {
        (pos.x, pos.y, pos.z)
    }
}

impl From<[i32; 3]> for BlockPos {
    fn from([x, y, z]: [i32; 3]) -> Self {
        BlockPos::new(x, y, z)
    }
}

impl From<BlockPos> for [i32; 3] {
    fn from(pos: BlockPos) -> Self {
        [pos.x, pos.y, pos.z]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position() {
        let xzs = [
            (-33554432, true),
            (-33554433, false),
            (33554431, true),
            (33554432, false),
            (0, true),
            (1, true),
            (-1, true),
        ];
        let ys = [
            (-2048, true),
            (-2049, false),
            (2047, true),
            (2048, false),
            (0, true),
            (1, true),
            (-1, true),
        ];

        let mut buf = [0; 8];

        for (x, x_valid) in xzs {
            for (y, y_valid) in ys {
                for (z, z_valid) in xzs {
                    let pos = BlockPos::new(x, y, z);
                    if x_valid && y_valid && z_valid {
                        pos.encode(&mut &mut buf[..]).unwrap();
                        assert_eq!(BlockPos::decode(&mut &buf[..]).unwrap(), pos);
                    } else {
                        assert!(pos.encode(&mut &mut buf[..]).is_err());
                    }
                }
            }
        }
    }
}
