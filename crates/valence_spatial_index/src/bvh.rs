use std::iter::FusedIterator;
use std::mem;

use approx::abs_diff_eq;
use rayon::iter::{
    IndexedParallelIterator, IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator,
};
use vek::{Aabb, Vec3};

use crate::{ray_box_intersect, Bounded3D, RaycastHit, SpatialIndex};

#[derive(Clone, Debug)]
pub struct Bvh<T> {
    internal_nodes: Vec<InternalNode>,
    leaf_nodes: Vec<T>,
    root: NodeIdx,
}

#[derive(Clone, Debug)]
struct InternalNode {
    bb: Aabb<f64>,
    left: NodeIdx,
    right: NodeIdx,
}

// TODO: we could use usize here to store more elements.
type NodeIdx = u32;

impl<T: Bounded3D + Send + Sync> Bvh<T> {
    pub fn new() -> Self {
        Self {
            internal_nodes: vec![],
            leaf_nodes: vec![],
            root: NodeIdx::MAX,
        }
    }

    pub fn rebuild(&mut self, leaves: impl IntoIterator<Item = T>) {
        self.internal_nodes.clear();
        self.leaf_nodes.clear();

        self.leaf_nodes.extend(leaves);

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

        let id = self.leaf_nodes[0].aabb();
        let scene_bounds = self
            .leaf_nodes
            .par_iter()
            .map(|l| l.aabb())
            .reduce(|| id, Aabb::union);

        self.root = rebuild_rec(
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

    pub fn iter(&self) -> impl ExactSizeIterator<Item = &T> + FusedIterator + Clone + '_ {
        self.leaf_nodes.iter()
    }

    pub fn iter_mut(&mut self) -> impl ExactSizeIterator<Item = &mut T> + FusedIterator + '_ {
        self.leaf_nodes.iter_mut()
    }

    pub fn par_iter(&self) -> impl IndexedParallelIterator<Item = &T> + Clone + '_ {
        self.leaf_nodes.par_iter()
    }

    pub fn par_iter_mut(&mut self) -> impl IndexedParallelIterator<Item = &mut T> + '_ {
        self.leaf_nodes.par_iter_mut()
    }
}

impl<T: Bounded3D + Send + Sync> Default for Bvh<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub enum Node<'a, T> {
    Internal(Internal<'a, T>),
    Leaf(&'a T),
}

impl<'a, T> Node<'a, T> {
    fn from_idx(bvh: &'a Bvh<T>, idx: NodeIdx) -> Self {
        if idx < bvh.internal_nodes.len() as NodeIdx {
            Self::Internal(Internal { bvh, idx })
        } else {
            Self::Leaf(&bvh.leaf_nodes[(idx - bvh.internal_nodes.len() as NodeIdx) as usize])
        }
    }
}

impl<T: Bounded3D> Bounded3D for Node<'_, T> {
    fn aabb(&self) -> Aabb<f64> {
        match self {
            Node::Internal(int) => int.aabb(),
            Node::Leaf(t) => t.aabb(),
        }
    }
}

impl<T> Clone for Node<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Node<'_, T> {}

#[derive(Debug)]
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
}

impl<T> Bounded3D for Internal<'_, T> {
    fn aabb(&self) -> Aabb<f64> {
        self.bvh.internal_nodes[self.idx as usize].bb
    }
}

impl<T> Clone for Internal<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Internal<'_, T> {}

fn rebuild_rec<T: Send + Bounded3D>(
    idx: NodeIdx,
    mut bounds: Aabb<f64>,
    internal_nodes: &mut [InternalNode],
    leaf_nodes: &mut [T],
    total_leaf_count: NodeIdx,
) -> (NodeIdx, Aabb<f64>) {
    debug_assert_eq!(leaf_nodes.len() - 1, internal_nodes.len());

    if leaf_nodes.len() == 1 {
        // Leaf node
        return (total_leaf_count - 1 + idx, leaf_nodes[0].aabb());
    }

    loop {
        debug_assert!(bounds.is_valid());
        let dims = bounds.max - bounds.min;

        let (mut split, bounds_left, bounds_right) = if dims.x >= dims.y && dims.x >= dims.z {
            let mid = middle(bounds.min.x, bounds.max.x);
            let [bounds_left, bounds_right] = bounds.split_at_x(mid);

            let p = partition(leaf_nodes, |l| {
                middle(l.aabb().min.x, l.aabb().max.x) <= mid
            });

            (p, bounds_left, bounds_right)
        } else if dims.y >= dims.x && dims.y >= dims.z {
            let mid = middle(bounds.min.y, bounds.max.y);
            let [bounds_left, bounds_right] = bounds.split_at_y(mid);

            let p = partition(leaf_nodes, |l| {
                middle(l.aabb().min.y, l.aabb().max.y) <= mid
            });

            (p, bounds_left, bounds_right)
        } else {
            let mid = middle(bounds.min.z, bounds.max.z);
            let [bounds_left, bounds_right] = bounds.split_at_z(mid);

            let p = partition(leaf_nodes, |l| {
                middle(l.aabb().min.z, l.aabb().max.z) <= mid
            });

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
                rebuild_rec(
                    idx,
                    bounds_left,
                    internal_left,
                    leaves_left,
                    total_leaf_count,
                )
            },
            || {
                rebuild_rec(
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

impl<O: Bounded3D + Send + Sync> SpatialIndex for Bvh<O> {
    type Object = O;

    fn query<C, F, T>(&self, mut collides: C, mut f: F) -> Option<T>
    where
        C: FnMut(Aabb<f64>) -> bool,
        F: FnMut(&O) -> Option<T>,
    {
        fn query_rec<C, F, O, T>(node: Node<O>, collides: &mut C, f: &mut F) -> Option<T>
        where
            C: FnMut(Aabb<f64>) -> bool,
            F: FnMut(&O) -> Option<T>,
            O: Bounded3D,
        {
            match node {
                Node::Internal(int) => {
                    let (bb, left, right) = int.split();

                    if collides(bb) {
                        query_rec(left, collides, f).or_else(|| query_rec(right, collides, f))
                    } else {
                        None
                    }
                }
                Node::Leaf(leaf) => {
                    if collides(leaf.aabb()) {
                        f(leaf)
                    } else {
                        None
                    }
                }
            }
        }

        query_rec(self.traverse()?, &mut collides, &mut f)
    }

    fn raycast<F>(&self, origin: Vec3<f64>, direction: Vec3<f64>, mut f: F) -> Option<RaycastHit<O>>
    where
        F: FnMut(RaycastHit<O>) -> bool,
    {
        fn raycast_rec<'a, O: Bounded3D>(
            node: Node<'a, O>,
            hit: &mut Option<RaycastHit<'a, O>>,
            near: f64,
            far: f64,
            origin: Vec3<f64>,
            direction: Vec3<f64>,
            f: &mut impl FnMut(RaycastHit<O>) -> bool,
        ) {
            if let Some(hit) = hit {
                if hit.near <= near {
                    return;
                }
            }

            match node {
                Node::Internal(int) => {
                    let (_, left, right) = int.split();

                    let int_left = ray_box_intersect(origin, direction, left.aabb());
                    let int_right = ray_box_intersect(origin, direction, right.aabb());

                    match (int_left, int_right) {
                        (Some((near_left, far_left)), Some((near_right, far_right))) => {
                            // Explore closest subtree first.
                            if near_left < near_right {
                                raycast_rec(left, hit, near_left, far_left, origin, direction, f);
                                raycast_rec(
                                    right, hit, near_right, far_right, origin, direction, f,
                                );
                            } else {
                                raycast_rec(
                                    right, hit, near_right, far_right, origin, direction, f,
                                );
                                raycast_rec(left, hit, near_left, far_left, origin, direction, f);
                            }
                        }
                        (Some((near, far)), None) => {
                            raycast_rec(left, hit, near, far, origin, direction, f)
                        }
                        (None, Some((near, far))) => {
                            raycast_rec(right, hit, near, far, origin, direction, f)
                        }
                        (None, None) => {}
                    }
                }
                Node::Leaf(leaf) => {
                    let this_hit = RaycastHit {
                        object: leaf,
                        near,
                        far,
                    };

                    if f(this_hit) {
                        *hit = Some(this_hit);
                    }
                }
            }
        }

        debug_assert!(
            direction.is_normalized(),
            "the ray direction must be normalized"
        );

        let root = self.traverse()?;
        let (near, far) = ray_box_intersect(origin, direction, root.aabb())?;

        let mut hit = None;
        raycast_rec(root, &mut hit, near, far, origin, direction, &mut f);
        hit
    }
}
