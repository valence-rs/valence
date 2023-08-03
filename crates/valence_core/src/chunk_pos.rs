use glam::DVec3;

use crate::block_pos::BlockPos;
use crate::protocol::{Decode, Encode};

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

include!(concat!(env!("OUT_DIR"), "/chunk_pos.rs"));

/// Represents the set of all chunk positions that a client can see, defined by
/// a center chunk position `pos` and view distance `dist`.
#[derive(Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct ChunkView {
    /// The center position of this chunk view.
    pub pos: ChunkPos,
    dist: u8,
}

impl ChunkView {
    /// Creates a new chunk view. `dist` is clamped to the range
    /// 0..[MAX_VIEW_DIST].
    pub const fn new(pos: ChunkPos, dist: u8) -> Self {
        Self {
            pos,
            dist: if dist > MAX_VIEW_DIST {
                MAX_VIEW_DIST
            } else {
                dist
            },
        }
    }

    pub const fn with_dist(self, dist: u8) -> Self {
        Self::new(self.pos, dist)
    }

    pub const fn dist(self) -> u8 {
        self.dist
    }

    pub const fn contains(self, pos: ChunkPos) -> bool {
        let true_dist = self.dist as u64 + EXTRA_VIEW_RADIUS as u64;
        self.pos.distance_squared(pos) <= true_dist * true_dist
    }

    /// Returns an iterator over all the chunk positions in this view. Positions
    /// are sorted by the distance to [`pos`](Self::pos) in ascending order.
    pub fn iter(self) -> impl DoubleEndedIterator<Item = ChunkPos> + ExactSizeIterator + Clone {
        CHUNK_VIEW_LUT[self.dist as usize]
            .iter()
            .map(move |&(x, z)| ChunkPos {
                x: self.pos.x + x as i32,
                z: self.pos.z + z as i32,
            })
    }

    /// Returns an iterator over all the chunk positions in `self`, excluding
    /// the positions that overlap with `other`. Positions are sorted by the
    /// distance to [`pos`](Self::pos) in ascending order.
    pub fn diff(self, other: Self) -> impl DoubleEndedIterator<Item = ChunkPos> + Clone {
        self.iter().filter(move |&p| !other.contains(p))
    }

    /// Returns a `(min, max)` tuple describing the tight axis-aligned bounding
    /// box for this view. All chunk positions in the view are contained in the
    /// bounding box.
    ///
    /// # Examples
    ///
    /// ```
    /// use valence_core::chunk_pos::{ChunkPos, ChunkView};
    ///
    /// let view = ChunkView::new(ChunkPos::new(5, -4), 16);
    /// let (min, max) = view.bounding_box();
    ///
    /// for pos in view.iter() {
    ///     assert!(pos.x >= min.x && pos.x <= max.x && pos.z >= min.z && pos.z <= max.z);
    /// }
    /// ```
    pub fn bounding_box(self) -> (ChunkPos, ChunkPos) {
        let r = self.dist as i32 + EXTRA_VIEW_RADIUS;

        (
            ChunkPos::new(self.pos.x - r, self.pos.z - r),
            ChunkPos::new(self.pos.x + r, self.pos.z + r),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn chunk_view_contains() {
        let view = ChunkView::new(ChunkPos::new(0, 0), 32);
        let positions = BTreeSet::from_iter(view.iter());

        for z in -64..64 {
            for x in -64..64 {
                let p = ChunkPos::new(x, z);
                assert_eq!(view.contains(p), positions.contains(&p), "{p:?}");
            }
        }
    }

    #[test]
    fn chunk_pos_round_trip_conv() {
        let p = ChunkPos::new(rand::random(), rand::random());

        assert_eq!(ChunkPos::from(<(i32, i32)>::from(p)), p);
        assert_eq!(ChunkPos::from(<[i32; 2]>::from(p)), p);
    }
}
