//! The [bounding volume hierarchy][bvh] contained in the [`SpatialIndex`]
//!
//! [bvh]: https://en.wikipedia.org/wiki/Bounding_volume_hierarchy
//! [`SpatialIndex`]: crate::spatial_index::SpatialIndex

use std::iter::FusedIterator;
use std::mem;

use approx::abs_diff_eq;
use rayon::iter::{
    IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
};
use vek::Aabb;

#[derive(Clone)]
pub struct Bvh<T> {
    internal_nodes: Vec<InternalNode>,
    leaf_nodes: Vec<LeafNode<T>>,
    root: NodeIdx,
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
    data: T,
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
            .extend(leaves.into_iter().map(|(id, bb)| LeafNode { bb, data: id }));

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

    pub fn traverse(&self) -> Option<Node<T>> {
        if !self.leaf_nodes.is_empty() {
            Some(Node::from_idx(self, self.root))
        } else {
            None
        }
    }

    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (&T, Aabb<f64>)> + FusedIterator + Clone + '_ {
        self.leaf_nodes.iter().map(|leaf| (&leaf.data, leaf.bb))
    }

    #[allow(dead_code)]
    pub fn iter_mut(
        &mut self,
    ) -> impl ExactSizeIterator<Item = (&mut T, Aabb<f64>)> + FusedIterator + '_ {
        self.leaf_nodes
            .iter_mut()
            .map(|leaf| (&mut leaf.data, leaf.bb))
    }

    pub fn par_iter(&self) -> impl IndexedParallelIterator<Item = (&T, Aabb<f64>)> + Clone + '_ {
        self.leaf_nodes.par_iter().map(|leaf| (&leaf.data, leaf.bb))
    }

    #[allow(dead_code)]
    pub fn par_iter_mut(
        &mut self,
    ) -> impl IndexedParallelIterator<Item = (&mut T, Aabb<f64>)> + '_ {
        self.leaf_nodes
            .par_iter_mut()
            .map(|leaf| (&mut leaf.data, leaf.bb))
    }
}

pub enum Node<'a, T> {
    Internal(Internal<'a, T>),
    Leaf { data: &'a T, bb: Aabb<f64> },
}

impl<'a, T> Node<'a, T> {
    fn from_idx(bvh: &'a Bvh<T>, idx: NodeIdx) -> Self {
        if idx < bvh.internal_nodes.len() as NodeIdx {
            Self::Internal(Internal { bvh, idx })
        } else {
            let leaf = &bvh.leaf_nodes[(idx - bvh.internal_nodes.len() as NodeIdx) as usize];
            Self::Leaf {
                data: &leaf.data,
                bb: leaf.bb,
            }
        }
    }

    pub fn bb(&self) -> Aabb<f64> {
        match self {
            Node::Internal(int) => int.bb(),
            Node::Leaf { bb, .. } => *bb,
        }
    }
}

pub struct Internal<'a, T> {
    bvh: &'a Bvh<T>,
    idx: NodeIdx,
}

impl<'a, T> Internal<'a, T> {
    pub fn split(self) -> (Aabb<f64>, Node<'a, T>, Node<'a, T>) {
        let internal = &self.bvh.internal_nodes[self.idx as usize];

        let bb = internal.bb;
        let left = Node::from_idx(self.bvh, internal.left);
        let right = Node::from_idx(self.bvh, internal.right);

        (bb, left, right)
    }

    pub fn bb(&self) -> Aabb<f64> {
        self.bvh.internal_nodes[self.idx as usize].bb
    }
}

fn build_rec<T: Send>(
    idx: NodeIdx,
    mut bounds: Aabb<f64>,
    internal_nodes: &mut [InternalNode],
    leaf_nodes: &mut [LeafNode<T>],
    total_leaf_count: NodeIdx,
) -> (NodeIdx, Aabb<f64>) {
    debug_assert_eq!(leaf_nodes.len() - 1, internal_nodes.len());

    if leaf_nodes.len() == 1 {
        // Leaf node
        return (total_leaf_count - 1 + idx, leaf_nodes[0].bb);
    }

    loop {
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
            if abs_diff_eq!(
                bounds_right.min,
                bounds_right.max,
                epsilon = f64::EPSILON * 100.0
            ) {
                split += 1;
            } else {
                bounds = bounds_right;
                continue;
            }
        } else if split == leaf_nodes.len() {
            if abs_diff_eq!(
                bounds_left.min,
                bounds_left.max,
                epsilon = f64::EPSILON * 100.0
            ) {
                split -= 1;
            } else {
                bounds = bounds_left;
                continue;
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

        break (idx + split as NodeIdx - 1, internal.bb);
    }
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
