use std::io::Write;
use std::ops::{Add, Sub};

use anyhow::bail;
use valence_math::DVec3;

use crate::chunk_pos::ChunkPos;
use crate::direction::Direction;
use crate::{Decode, Encode};

/// Represents an absolute block position in world space.
#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    /// Constructs a new block position.
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Returns the block position a point in world space is contained within.
    pub fn from_pos(pos: DVec3) -> Self {
        Self {
            x: pos.x.floor() as i32,
            y: pos.y.floor() as i32,
            z: pos.z.floor() as i32,
        }
    }

    pub const fn to_chunk_pos(self) -> ChunkPos {
        ChunkPos::from_block_pos(self)
    }

    /// Get a new [`BlockPos`] that is adjacent to this position in `dir`
    /// direction.
    ///
    /// ```
    /// use valence_protocol::{BlockPos, Direction};
    ///
    /// let pos = BlockPos::new(0, 0, 0);
    /// let adj = pos.get_in_direction(Direction::South);
    /// assert_eq!(adj, BlockPos::new(0, 0, 1));
    /// ```
    pub const fn get_in_direction(self, dir: Direction) -> BlockPos {
        match dir {
            Direction::Down => BlockPos::new(self.x, self.y - 1, self.z),
            Direction::Up => BlockPos::new(self.x, self.y + 1, self.z),
            Direction::North => BlockPos::new(self.x, self.y, self.z - 1),
            Direction::South => BlockPos::new(self.x, self.y, self.z + 1),
            Direction::West => BlockPos::new(self.x - 1, self.y, self.z),
            Direction::East => BlockPos::new(self.x + 1, self.y, self.z),
        }
    }
}

impl Encode for BlockPos {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        match (self.x, self.y, self.z) {
            (-0x2000000..=0x1ffffff, -0x800..=0x7ff, -0x2000000..=0x1ffffff) => {
                let (x, y, z) = (self.x as u64, self.y as u64, self.z as u64);
                (x << 38 | z << 38 >> 26 | y & 0xfff).encode(w)
            }
            _ => bail!("out of range: {self:?}"),
        }
    }
}

impl Decode<'_> for BlockPos {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
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

impl Add for BlockPos {
    type Output = BlockPos;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }
}

impl Sub for BlockPos {
    type Output = BlockPos;

    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
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
