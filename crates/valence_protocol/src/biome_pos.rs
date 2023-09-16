use valence_math::DVec3;

use crate::BlockPos;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default, Debug)]
pub struct BiomePos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BiomePos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

impl From<BlockPos> for BiomePos {
    fn from(pos: BlockPos) -> Self {
        Self {
            x: pos.x.div_euclid(4),
            y: pos.y.div_euclid(4),
            z: pos.z.div_euclid(4),
        }
    }
}

impl From<DVec3> for BiomePos {
    fn from(pos: DVec3) -> Self {
        Self {
            x: (pos.x / 4.0).floor() as i32,
            y: (pos.y / 4.0).floor() as i32,
            z: (pos.z / 4.0).floor() as i32,
        }
    }
}
