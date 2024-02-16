use valence_generated::chunk_view::{CHUNK_VIEW_LUT, EXTRA_VIEW_RADIUS, MAX_VIEW_DIST};
use valence_protocol::ChunkPos;

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
    /// use valence_server::{ChunkPos, ChunkView};
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
}
