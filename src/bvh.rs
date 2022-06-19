use std::mem;

use approx::relative_eq;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use vek::Aabb;

#[derive(Clone)]
pub struct Bvh<T> {
    internal_nodes: Vec<InternalNode>,
    leaf_nodes: Vec<LeafNode<T>>,
    root: NodeIdx,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TraverseStep<T> {
    Miss,
    Hit,
    Return(T),
}

#[derive(Clone)]
struct InternalNode {
    bb: Aabb<f64>,
    left: NodeIdx,
    right: NodeIdx,
}

#[derive(Clone)]
struct LeafNode<T> {
    bb: Aabb<f64>,
    id: T,
}

// TODO: we could use usize here to store more elements.
type NodeIdx = u32;

impl<T: Send + Sync> Bvh<T> {
    pub fn new() -> Self {
        Self {
            internal_nodes: Vec::new(),
            leaf_nodes: Vec::new(),
            root: NodeIdx::MAX,
        }
    }

    pub fn build(&mut self, leaves: impl IntoIterator<Item = (T, Aabb<f64>)>) {
        self.leaf_nodes.clear();
        self.internal_nodes.clear();

        self.leaf_nodes
            .extend(leaves.into_iter().map(|(id, bb)| LeafNode { bb, id }));

        let leaf_count = self.leaf_nodes.len();

        if leaf_count == 0 {
            return;
        }

        self.internal_nodes.reserve_exact(leaf_count - 1);
        self.internal_nodes.resize(
            leaf_count - 1,
            InternalNode {
                bb: Aabb::default(),
                left: NodeIdx::MAX,
                right: NodeIdx::MAX,
            },
        );

        if NodeIdx::try_from(leaf_count)
            .ok()
            .and_then(|count| count.checked_add(count - 1))
            .is_none()
        {
            panic!("too many elements in BVH");
        }

        let id = self.leaf_nodes[0].bb;
        let scene_bounds = self
            .leaf_nodes
            .par_iter()
            .map(|l| l.bb)
            .reduce(|| id, Aabb::union);

        self.root = build_rec(
            0,
            scene_bounds,
            &mut self.internal_nodes,
            &mut self.leaf_nodes,
            leaf_count as NodeIdx,
        )
        .0;

        debug_assert_eq!(self.internal_nodes.len(), self.leaf_nodes.len() - 1);
    }

    pub fn traverse<F, U>(&self, mut f: F) -> Option<U>
    where
        F: FnMut(Option<&T>, Aabb<f64>) -> TraverseStep<U>,
    {
        if !self.leaf_nodes.is_empty() {
            self.traverse_rec(self.root, &mut f)
        } else {
            None
        }
    }

    fn traverse_rec<F, U>(&self, idx: NodeIdx, f: &mut F) -> Option<U>
    where
        F: FnMut(Option<&T>, Aabb<f64>) -> TraverseStep<U>,
    {
        if idx < self.internal_nodes.len() as NodeIdx {
            let internal = &self.internal_nodes[idx as usize];

            match f(None, internal.bb) {
                TraverseStep::Miss => None,
                TraverseStep::Hit => self
                    .traverse_rec(internal.left, f)
                    .or_else(|| self.traverse_rec(internal.right, f)),
                TraverseStep::Return(u) => Some(u),
            }
        } else {
            let leaf = &self.leaf_nodes[(idx - self.internal_nodes.len() as NodeIdx) as usize];

            match f(Some(&leaf.id), leaf.bb) {
                TraverseStep::Miss | TraverseStep::Hit => None,
                TraverseStep::Return(u) => Some(u),
            }
        }
    }
}

fn build_rec<T: Send>(
    idx: NodeIdx,
    bounds: Aabb<f64>,
    internal_nodes: &mut [InternalNode],
    leaf_nodes: &mut [LeafNode<T>],
    total_leaf_count: NodeIdx,
) -> (NodeIdx, Aabb<f64>) {
    debug_assert_eq!(leaf_nodes.len() - 1, internal_nodes.len());

    if leaf_nodes.len() == 1 {
        // Leaf node
        return (total_leaf_count - 1 + idx, leaf_nodes[0].bb);
    }

    debug_assert!(bounds.is_valid());
    let dims = bounds.max - bounds.min;

    let (mut split, bounds_left, bounds_right) = if dims.x >= dims.y && dims.x >= dims.z {
        let mid = middle(bounds.min.x, bounds.max.x);
        let [bounds_left, bounds_right] = bounds.split_at_x(mid);

        let p = partition(leaf_nodes, |l| middle(l.bb.min.x, l.bb.max.x) <= mid);

        (p, bounds_left, bounds_right)
    } else if dims.y >= dims.x && dims.y >= dims.z {
        let mid = middle(bounds.min.y, bounds.max.y);
        let [bounds_left, bounds_right] = bounds.split_at_y(mid);

        let p = partition(leaf_nodes, |l| middle(l.bb.min.y, l.bb.max.y) <= mid);

        (p, bounds_left, bounds_right)
    } else {
        let mid = middle(bounds.min.z, bounds.max.z);
        let [bounds_left, bounds_right] = bounds.split_at_z(mid);

        let p = partition(leaf_nodes, |l| middle(l.bb.min.z, l.bb.max.z) <= mid);

        (p, bounds_left, bounds_right)
    };

    // Check if one of the halves is empty. (We can't have empty nodes)
    // Also take care to handle the edge case of overlapping points.
    if split == 0 {
        if relative_eq!(bounds_right.min, bounds_right.max) {
            split += 1;
        } else {
            return build_rec(
                idx,
                bounds_right,
                internal_nodes,
                leaf_nodes,
                total_leaf_count,
            );
        }
    } else if split == leaf_nodes.len() {
        if relative_eq!(bounds_left.min, bounds_left.max) {
            split -= 1;
        } else {
            return build_rec(
                idx,
                bounds_left,
                internal_nodes,
                leaf_nodes,
                total_leaf_count,
            );
        }
    }

    let (leaves_left, leaves_right) = leaf_nodes.split_at_mut(split);

    let (internal_left, internal_right) = internal_nodes.split_at_mut(split);
    let (internal, internal_left) = internal_left.split_last_mut().unwrap();

    let ((left, bounds_left), (right, bounds_right)) = rayon::join(
        || {
            build_rec(
                idx,
                bounds_left,
                internal_left,
                leaves_left,
                total_leaf_count,
            )
        },
        || {
            build_rec(
                idx + split as NodeIdx,
                bounds_right,
                internal_right,
                leaves_right,
                total_leaf_count,
            )
        },
    );

    internal.bb = bounds_left.union(bounds_right);
    internal.left = left;
    internal.right = right;

    (idx + split as NodeIdx - 1, internal.bb)
}

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

fn middle(a: f64, b: f64) -> f64 {
    (a + b) / 2.0
}

impl<T: Send + Sync> Default for Bvh<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let mut bvh = Bvh::new();

        bvh.traverse(|_, _| TraverseStep::Return(()));
        bvh.build([]);

        bvh.build([(5, Aabb::default())]);
        bvh.traverse(|_, _| TraverseStep::Return(()));
    }

    #[test]
    fn overlapping() {
        let mut bvh = Bvh::new();

        bvh.build([
            ((), Aabb::default()),
            ((), Aabb::default()),
            ((), Aabb::default()),
            ((), Aabb::default()),
            ((), Aabb::new_empty(5.0.into())),
        ]);

        bvh.traverse(|_, _| TraverseStep::Return(()));
    }
}
