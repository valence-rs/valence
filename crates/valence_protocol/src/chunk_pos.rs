use valence_math::DVec3;

use crate::block_pos::BlockPos;
use crate::{Decode, Encode};

/// The X and Z position of a chunk.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Debug, Encode, Decode)]
pub struct ChunkPos {
    /// The X position of the chunk.
    pub x: i32,
    /// The Z position of the chunk.
    pub z: i32,
}

impl ChunkPos {
    /// Constructs a new chunk position.
    pub const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    /// Constructs a chunk position from a position in world space. Only the `x`
    /// and `z` components are used.
    pub fn from_pos(pos: DVec3) -> Self {
        Self::new((pos.x / 16.0).floor() as i32, (pos.z / 16.0).floor() as i32)
    }

    pub const fn from_block_pos(pos: BlockPos) -> Self {
        Self::new(pos.x.div_euclid(16), pos.z.div_euclid(16))
    }

    pub const fn distance_squared(self, other: Self) -> u64 {
        let diff_x = other.x as i64 - self.x as i64;
        let diff_z = other.z as i64 - self.z as i64;

        (diff_x * diff_x + diff_z * diff_z) as u64
    }
}

impl From<(i32, i32)> for ChunkPos {
    fn from((x, z): (i32, i32)) -> Self {
        Self { x, z }
    }
}

impl From<ChunkPos> for (i32, i32) {
    fn from(pos: ChunkPos) -> Self {
        (pos.x, pos.z)
    }
}

impl From<[i32; 2]> for ChunkPos {
    fn from([x, z]: [i32; 2]) -> Self {
        Self { x, z }
    }
}

impl From<ChunkPos> for [i32; 2] {
    fn from(pos: ChunkPos) -> Self {
        [pos.x, pos.z]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_pos_round_trip_conv() {
        let p = ChunkPos::new(rand::random(), rand::random());

        assert_eq!(ChunkPos::from(<(i32, i32)>::from(p)), p);
        assert_eq!(ChunkPos::from(<[i32; 2]>::from(p)), p);
    }
}
