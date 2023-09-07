use std::fmt;
use std::io::Write;

use anyhow::bail;
use bitfield_struct::bitfield;
use derive_more::From;
use thiserror::Error;
use valence_math::{DVec3, IVec3};

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
    pub const fn get_in_direction(self, dir: Direction) -> Self {
        match dir {
            Direction::Down => BlockPos::new(self.x, self.y - 1, self.z),
            Direction::Up => BlockPos::new(self.x, self.y + 1, self.z),
            Direction::North => BlockPos::new(self.x, self.y, self.z - 1),
            Direction::South => BlockPos::new(self.x, self.y, self.z + 1),
            Direction::West => BlockPos::new(self.x - 1, self.y, self.z),
            Direction::East => BlockPos::new(self.x + 1, self.y, self.z),
        }
    }

    pub const fn offset(self, x: i32, y: i32, z: i32) -> Self {
        Self::new(self.x + x, self.y + y, self.z + z)
    }

    pub const fn packed(self) -> Result<PackedBlockPos, Error> {
        match (self.x, self.y, self.z) {
            (-0x2000000..=0x1ffffff, -0x800..=0x7ff, -0x2000000..=0x1ffffff) => {
                Ok(PackedBlockPos::new()
                    .with_x(self.x)
                    .with_y(self.y)
                    .with_z(self.z))
            }
            _ => Err(Error(self)),
        }
    }
}

#[bitfield(u64)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Encode, Decode)]
pub struct PackedBlockPos {
    #[bits(12)]
    pub y: i32,
    #[bits(26)]
    pub z: i32,
    #[bits(26)]
    pub x: i32,
}

impl Encode for BlockPos {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        match self.packed() {
            Ok(p) => p.encode(w),
            Err(e) => bail!("{e}: {self}"),
        }
    }
}

impl Decode<'_> for BlockPos {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        PackedBlockPos::decode(r).map(Into::into)
    }
}

impl From<PackedBlockPos> for BlockPos {
    fn from(p: PackedBlockPos) -> Self {
        Self {
            x: p.x(),
            y: p.y(),
            z: p.z(),
        }
    }
}

impl TryFrom<BlockPos> for PackedBlockPos {
    type Error = Error;

    fn try_from(pos: BlockPos) -> Result<Self, Self::Error> {
        pos.packed()
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Error, From)]
#[error("block position of {0} is out of range")]
pub struct Error(pub BlockPos);

impl From<DVec3> for BlockPos {
    fn from(pos: DVec3) -> Self {
        Self {
            x: pos.x.floor() as i32,
            y: pos.y.floor() as i32,
            z: pos.z.floor() as i32,
        }
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

impl Add<IVec3> for BlockPos {
    type Output = Self;

    fn add(self, rhs: IVec3) -> Self::Output {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub<IVec3> for BlockPos {
    type Output = Self;

    fn sub(self, rhs: IVec3) -> Self::Output {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Add<BlockPos> for IVec3 {
    type Output = BlockPos;

    fn add(self, rhs: BlockPos) -> Self::Output {
        BlockPos::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub<BlockPos> for IVec3 {
    type Output = BlockPos;

    fn sub(self, rhs: BlockPos) -> Self::Output {
        BlockPos::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl fmt::Display for BlockPos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Display the block position as a tuple.
        fmt::Debug::fmt(&(self.x, self.y, self.z), f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_position() {
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

        for (x, x_valid) in xzs {
            for (y, y_valid) in ys {
                for (z, z_valid) in xzs {
                    let pos = BlockPos::new(x, y, z);
                    if x_valid && y_valid && z_valid {
                        let c = pos.packed().unwrap();
                        assert_eq!((c.x(), c.y(), c.z()), (pos.x, pos.y, pos.z));
                    } else {
                        assert_eq!(pos.packed(), Err(Error(pos)));
                    }
                }
            }
        }
    }
}
