use std::iter::FusedIterator;

use valence_protocol::BlockPos;

/// The X and Z position of a chunk in a world.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Debug)]
pub struct ChunkPos {
    /// The X position of the chunk.
    pub x: i32,
    /// The Z position of the chunk.
    pub z: i32,
}

const EXTRA_VIEW_RADIUS: i32 = 2;

impl ChunkPos {
    /// Constructs a new chunk position.
    pub const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    /// Takes an X and Z position in world space and returns the chunk position
    /// containing the point.
    pub fn at(x: f64, z: f64) -> Self {
        Self::new((x / 16.0).floor() as i32, (z / 16.0).floor() as i32)
    }

    /// Checks if two chunk positions are within a view distance (render
    /// distance) of each other such that a client standing in `self` would
    /// be able to see `other`.
    #[inline]
    pub fn is_in_view(self, other: Self, view_dist: u8) -> bool {
        let dist = view_dist as i64 + EXTRA_VIEW_RADIUS as i64;

        let diff_x = other.x as i64 - self.x as i64;
        let diff_z = other.z as i64 - self.z as i64;

        diff_x * diff_x + diff_z * diff_z <= dist * dist
    }

    /// Returns an iterator over all chunk positions within a view distance
    /// centered on `self`. The `self` position is included in the output.
    pub fn in_view(self, view_dist: u8) -> impl FusedIterator<Item = Self> {
        let dist = view_dist as i32 + EXTRA_VIEW_RADIUS;

        (self.z - dist..=self.z + dist)
            .flat_map(move |z| (self.x - dist..=self.x + dist).map(move |x| Self { x, z }))
            .filter(move |&p| self.is_in_view(p, view_dist))
    }

    // `in_view` wasn't optimizing well so we're using this for now.
    #[inline(always)]
    pub(crate) fn try_for_each_in_view<F>(self, view_dist: u8, mut f: F) -> anyhow::Result<()>
    where
        F: FnMut(ChunkPos) -> anyhow::Result<()>,
    {
        let dist = view_dist as i32 + EXTRA_VIEW_RADIUS;

        for z in self.z - dist..=self.z + dist {
            for x in self.x - dist..=self.x + dist {
                let p = Self { x, z };
                if self.is_in_view(p, view_dist) {
                    f(p)?;
                }
            }
        }

        Ok(())
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

impl From<BlockPos> for ChunkPos {
    fn from(pos: BlockPos) -> Self {
        Self::new(pos.x.div_euclid(16), pos.z.div_euclid(16))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_in_view() {
        let center = ChunkPos::new(42, 24);

        for dist in 2..=32 {
            for pos in center.in_view(dist) {
                assert!(center.is_in_view(pos, dist));
                assert!(pos.is_in_view(center, dist));
            }
        }
    }
}
