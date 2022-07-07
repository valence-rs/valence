use crate::block::BlockPos;

/// The X and Z position of a chunk in a world.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ChunkPos {
    /// The X position of the chunk.
    pub x: i32,
    /// The Z position of the chunk.
    pub z: i32,
}

impl ChunkPos {
    pub const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    pub fn at(x: f64, z: f64) -> Self {
        Self::new((x / 16.0).floor() as i32, (z / 16.0).floor() as i32)
    }
}

impl From<(i32, i32)> for ChunkPos {
    fn from((x, z): (i32, i32)) -> Self {
        ChunkPos { x, z }
    }
}

impl Into<(i32, i32)> for ChunkPos {
    fn into(self) -> (i32, i32) {
        (self.x, self.z)
    }
}

impl From<[i32; 2]> for ChunkPos {
    fn from([x, z]: [i32; 2]) -> Self {
        (x, z).into()
    }
}

impl Into<[i32; 2]> for ChunkPos {
    fn into(self) -> [i32; 2] {
        [self.x, self.z]
    }
}

impl From<BlockPos> for ChunkPos {
    fn from(pos: BlockPos) -> Self {
        Self::new(pos.x.div_euclid(16), pos.z.div_euclid(16))
    }
}
