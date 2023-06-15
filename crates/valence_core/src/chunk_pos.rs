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

const EXTRA_VIEW_RADIUS: i32 = 2;

impl ChunkPos {
    /// Constructs a new chunk position.
    pub const fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    /// Constructs a chunk position from a position in world space. Only the `x`
    /// and `z` components are used.
    pub fn from_dvec3(pos: DVec3) -> Self {
        Self::at(pos.x, pos.z)
    }

    pub fn from_block_pos(pos: BlockPos) -> Self {
        Self::new(pos.x.div_euclid(16), pos.z.div_euclid(16))
    }

    /// Takes an X and Z position in world space and returns the chunk position
    /// containing the point.
    pub fn at(x: f64, z: f64) -> Self {
        Self::new((x / 16.0).floor() as i32, (z / 16.0).floor() as i32)
    }

    pub fn distance_squared(self, other: Self) -> u64 {
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

/// Represents the set of all chunk positions that a client can see, defined by
/// a center chunk position `pos` and view distance `dist`.
#[derive(Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct ChunkView {
    pub pos: ChunkPos,
    pub dist: u8,
}

impl ChunkView {
    #[inline]
    pub fn new(pos: impl Into<ChunkPos>, dist: u8) -> Self {
        Self {
            pos: pos.into(),
            dist,
        }
    }

    #[must_use]
    pub fn with_pos(mut self, pos: impl Into<ChunkPos>) -> Self {
        self.pos = pos.into();
        self
    }

    #[must_use]
    pub fn with_dist(mut self, dist: u8) -> Self {
        self.dist = dist;
        self
    }

    #[inline]
    pub fn contains(self, pos: ChunkPos) -> bool {
        let true_dist = self.dist as u64 + EXTRA_VIEW_RADIUS as u64;
        self.pos.distance_squared(pos) <= true_dist * true_dist
    }

    /// Returns an iterator over all the chunk positions in this view.
    pub fn iter(self) -> impl Iterator<Item = ChunkPos> {
        let true_dist = self.dist as i32 + EXTRA_VIEW_RADIUS;

        (self.pos.z - true_dist..=self.pos.z + true_dist)
            .flat_map(move |z| {
                (self.pos.x - true_dist..=self.pos.x + true_dist).map(move |x| ChunkPos { x, z })
            })
            .filter(move |&p| self.contains(p))
    }

    pub fn diff(self, other: Self) -> impl Iterator<Item = ChunkPos> {
        self.iter().filter(move |&p| !other.contains(p))
    }

    // The foreach-based methods are optimizing better than the iterator ones.

    #[inline]
    pub fn for_each(self, mut f: impl FnMut(ChunkPos)) {
        let true_dist = self.dist as i32 + EXTRA_VIEW_RADIUS;

        for z in self.pos.z - true_dist..=self.pos.z + true_dist {
            for x in self.pos.x - true_dist..=self.pos.x + true_dist {
                let p = ChunkPos { x, z };
                if self.contains(p) {
                    f(p);
                }
            }
        }
    }

    #[inline]
    pub fn diff_for_each(self, other: Self, mut f: impl FnMut(ChunkPos)) {
        self.for_each(|p| {
            if !other.contains(p) {
                f(p);
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn chunk_view_for_each_and_iter() {
        let pos = ChunkPos::new(42, 24);

        for dist in 2..=32 {
            let mut positions = vec![];

            let view = ChunkView { pos, dist };

            view.for_each(|pos| {
                positions.push(pos);
                assert!(view.contains(pos))
            });

            for (i, pos) in view.iter().enumerate() {
                assert_eq!(positions[i], pos);
                assert!(view.contains(pos));
            }
        }
    }

    #[test]
    fn chunk_view_contains() {
        let view = ChunkView::new([0, 0], 16);
        let positions = BTreeSet::from_iter(view.iter());

        for z in -64..64 {
            for x in -64..64 {
                let p = ChunkPos::new(x, z);
                assert_eq!(view.contains(p), positions.contains(&p));
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
