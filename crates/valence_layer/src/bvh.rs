use std::mem;
use std::ops::Range;

use valence_core::chunk_pos::{ChunkPos, ChunkView};

#[derive(Clone, Debug)]
pub struct ChunkBvh<T, const MAX_SURFACE_AREA: i32 = { 8 * 4 }> {
    nodes: Vec<Node>,
    values: Vec<T>,
}

impl<T, const MAX_SURFACE_AREA: i32> Default for ChunkBvh<T, MAX_SURFACE_AREA> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
enum Node {
    Internal {
        bounds: Aabb,
        left: NodeIdx,
        right: NodeIdx,
    },
    Leaf {
        bounds: Aabb,
        /// Range of values in the values array.
        values: Range<NodeIdx>,
    },
}

#[cfg(test)]
impl Node {
    fn bounds(&self) -> Aabb {
        match self {
            Node::Internal { bounds, .. } => *bounds,
            Node::Leaf { bounds, .. } => *bounds,
        }
    }
}

type NodeIdx = u32;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
struct Aabb {
    min: ChunkPos,
    max: ChunkPos,
}

impl Aabb {
    fn point(pos: ChunkPos) -> Self {
        Self { min: pos, max: pos }
    }

    /// Sum of side lengths.
    fn surface_area(self) -> i32 {
        (self.length_x() + self.length_z()) * 2
    }

    /// Returns the smallest AABB containing `self` and `other`.
    fn union(self, other: Self) -> Self {
        Self {
            min: ChunkPos::new(self.min.x.min(other.min.x), self.min.z.min(other.min.z)),
            max: ChunkPos::new(self.max.x.max(other.max.x), self.max.z.max(other.max.z)),
        }
    }

    fn length_x(self) -> i32 {
        self.max.x - self.min.x
    }

    fn length_z(self) -> i32 {
        self.max.z - self.min.z
    }

    fn intersects(self, other: Self) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }
}

pub trait GetChunkPos {
    fn chunk_pos(&self) -> ChunkPos;
}

impl GetChunkPos for ChunkPos {
    fn chunk_pos(&self) -> ChunkPos {
        *self
    }
}

impl<T, const MAX_SURFACE_AREA: i32> ChunkBvh<T, MAX_SURFACE_AREA> {
    pub fn new() -> Self {
        assert!(MAX_SURFACE_AREA > 0);

        Self {
            nodes: vec![],
            values: vec![],
        }
    }
}

impl<T: GetChunkPos, const MAX_SURFACE_AREA: i32> ChunkBvh<T, MAX_SURFACE_AREA> {
    pub fn build(&mut self, items: impl IntoIterator<Item = T>) {
        self.nodes.clear();
        self.values.clear();

        self.values.extend(items);

        if let Some(bounds) = value_bounds(&self.values) {
            self.build_rec(bounds, 0..self.values.len());
        }
    }

    fn build_rec(&mut self, bounds: Aabb, value_range: Range<usize>) {
        if bounds.surface_area() <= MAX_SURFACE_AREA {
            self.nodes.push(Node::Leaf {
                bounds,
                values: value_range.start as u32..value_range.end as u32,
            });

            return;
        }

        let values = &mut self.values[value_range.clone()];

        // Determine splitting axis based on the side that's longer. Then split along
        // the spatial midpoint. We could use a more advanced heuristic like SAH,
        // but it probably doesn't matter here.

        let point = if bounds.length_x() >= bounds.length_z() {
            // Split on Z axis.

            let mid = middle(bounds.min.x, bounds.max.x);
            partition(values, |v| v.chunk_pos().x >= mid)
        } else {
            // Split on X axis.

            let mid = middle(bounds.min.z, bounds.max.z);
            partition(values, |v| v.chunk_pos().z >= mid)
        };

        let left_range = value_range.start..value_range.start + point;
        let right_range = left_range.end..value_range.end;

        let left_bounds =
            value_bounds(&self.values[left_range.clone()]).expect("left half should be nonempty");

        let right_bounds =
            value_bounds(&self.values[right_range.clone()]).expect("right half should be nonempty");

        self.build_rec(left_bounds, left_range);
        let left_idx = (self.nodes.len() - 1) as NodeIdx;

        self.build_rec(right_bounds, right_range);
        let right_idx = (self.nodes.len() - 1) as NodeIdx;

        self.nodes.push(Node::Internal {
            bounds,
            left: left_idx,
            right: right_idx,
        });
    }

    pub fn query(&self, view: ChunkView, mut f: impl FnMut(&T)) {
        if let Some(root) = self.nodes.last() {
            let (min, max) = view.bounding_box();
            self.query_rec(root, view, Aabb { min, max }, &mut f);
        }
    }

    fn query_rec(&self, node: &Node, view: ChunkView, view_aabb: Aabb, f: &mut impl FnMut(&T)) {
        match node {
            Node::Internal {
                bounds,
                left,
                right,
            } => {
                if bounds.intersects(view_aabb) {
                    self.query_rec(&self.nodes[*left as usize], view, view_aabb, f);
                    self.query_rec(&self.nodes[*right as usize], view, view_aabb, f);
                }
            }
            Node::Leaf { bounds, values } => {
                if bounds.intersects(view_aabb) {
                    for val in &self.values[values.start as usize..values.end as usize] {
                        if view.contains(val.chunk_pos()) {
                            f(val)
                        }
                    }
                }
            }
        }
    }

    pub fn shrink_to_fit(&mut self) {
        self.nodes.shrink_to_fit();
        self.values.shrink_to_fit();
    }

    #[cfg(test)]
    fn check_invariants(&self) {
        if let Some(root) = self.nodes.last() {
            self.check_invariants_rec(root);
        }
    }

    #[cfg(test)]
    fn check_invariants_rec(&self, node: &Node) {
        match node {
            Node::Internal {
                bounds,
                left,
                right,
            } => {
                let left = &self.nodes[*left as usize];
                let right = &self.nodes[*right as usize];

                assert_eq!(left.bounds().union(right.bounds()), *bounds);

                self.check_invariants_rec(left);
                self.check_invariants_rec(right);
            }
            Node::Leaf {
                bounds: leaf_bounds,
                values,
            } => {
                let bounds = value_bounds(&self.values[values.start as usize..values.end as usize])
                    .expect("leaf should be nonempty");

                assert_eq!(*leaf_bounds, bounds);
            }
        }
    }
}

fn value_bounds<T: GetChunkPos>(values: &[T]) -> Option<Aabb> {
    values
        .iter()
        .map(|v| Aabb::point(v.chunk_pos()))
        .reduce(Aabb::union)
}

fn middle(min: i32, max: i32) -> i32 {
    // Cast to i64 to avoid intermediate overflow.
    ((min as i64 + max as i64) / 2) as i32
}

/// Partitions the slice in place and returns the partition point. Why this
/// isn't in Rust's stdlib I don't know.
fn partition<T>(s: &mut [T], mut pred: impl FnMut(&T) -> bool) -> usize {
    let mut it = s.iter_mut();
    let mut true_count = 0;

    while let Some(head) = it.find(|x| {
        if pred(x) {
            true_count += 1;
            false
        } else {
            true
        }
    }) {
        if let Some(tail) = it.rfind(|x| pred(x)) {
            mem::swap(head, tail);
            true_count += 1;
        } else {
            break;
        }
    }
    true_count
}

#[cfg(test)]
mod tests {
    use rand::Rng;

    use super::*;

    #[test]
    fn partition_middle() {
        let mut arr = [2, 3, 4, 5];
        let mid = middle(arr[0], arr[arr.len() - 1]);

        let point = partition(&mut arr, |&x| mid >= x);

        assert_eq!(point, 2);
        assert_eq!(&arr[..point], &[2, 3]);
        assert_eq!(&arr[point..], &[4, 5]);
    }

    #[test]
    fn query_visits_correct_nodes() {
        let mut bvh = ChunkBvh::<ChunkPos>::new();

        let mut positions = vec![];

        let size = 500;
        let mut rng = rand::thread_rng();

        // Create a bunch of positions in a large area.
        for _ in 0..100_000 {
            positions.push(ChunkPos {
                x: rng.gen_range(-size / 2..size / 2),
                z: rng.gen_range(-size / 2..size / 2),
            });
        }

        // Put the view in the center of that area.
        let view = ChunkView::new(ChunkPos::default(), 32);

        let mut viewed_positions = vec![];

        // Create a list of positions the view contains.
        for &pos in &positions {
            if view.contains(pos) {
                viewed_positions.push(pos);
            }
        }

        bvh.build(positions);

        bvh.check_invariants();

        // Check that we traverse exactly the positions that we know the view can see.

        bvh.query(view, |pos| {
            let idx = viewed_positions.iter().position(|p| p == pos).expect("😔");
            viewed_positions.remove(idx);
        });

        assert!(viewed_positions.is_empty());
    }
}
