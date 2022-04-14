// TODO: rename to BlockPos and represent internally as [i32; 3].

use std::io::{Read, Write};

use anyhow::bail;
use glm::{IVec3, Scalar, TVec3};
use num::cast::AsPrimitive;

use crate::glm;
use crate::protocol::{Decode, Encode};

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, Debug)]
pub struct BlockPos(pub IVec3);

impl BlockPos {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self(glm::vec3(x, y, z))
    }

    pub fn from_vec3(vec: TVec3<impl Scalar + AsPrimitive<i32>>) -> Self {
        Self(vec.map(|n| n.as_()))
    }
}

impl<T: Scalar + Into<i32>> From<TVec3<T>> for BlockPos {
    fn from(vec: TVec3<T>) -> Self {
        Self(vec.map(|n| n.into()))
    }
}

impl Encode for BlockPos {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        match (self.0.x, self.0.y, self.0.z) {
            (-0x2000000..=0x1ffffff, -0x800..=0x7ff, -0x2000000..=0x1ffffff) => {
                let (x, y, z) = (self.0.x as u64, self.0.y as u64, self.0.z as u64);
                (x << 38 | z << 38 >> 26 | y & 0xfff).encode(w)
            }
            _ => bail!("block position {} is out of range", self.0),
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
        Ok(Self(glm::vec3(x as i32, y as i32, z as i32)))
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
                    let pos = BlockPos(glm::vec3(x, y, z));
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
