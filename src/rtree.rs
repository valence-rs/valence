use std::mem;

#[cfg(test)]
use approx::relative_eq;
use arrayvec::ArrayVec;
use ordered_float::OrderedFloat;
use vek::Aabb;

pub struct RTree<T, const MIN: usize, const MAX: usize> {
    root: Node<T, MIN, MAX>,
    // The bufs are put here to reuse their allocations.
    internal_split_buf: InternalBuf<T, MIN, MAX>,
    leaf_split_buf: LeafBuf<T>,
    reinsert_buf: LeafBuf<T>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QueryAction {
    Continue,
    Break,
}

type InternalBuf<T, const MIN: usize, const MAX: usize> = Vec<(Box<Node<T, MIN, MAX>>, Aabb<f64>)>;
type LeafBuf<T> = Vec<(T, Aabb<f64>)>;

enum Node<T, const MIN: usize, const MAX: usize> {
    Internal(ArrayVec<(Box<Node<T, MIN, MAX>>, Aabb<f64>), MAX>),
    Leaf(ArrayVec<(T, Aabb<f64>), MAX>),
}

impl<T, const MIN: usize, const MAX: usize> RTree<T, MIN, MAX> {
    pub fn new() -> Self {
        assert!(
            MIN >= 2 && MIN <= MAX / 2 && MAX >= 2,
            "invalid R-Tree configuration"
        );

        Self {
            root: Node::Leaf(ArrayVec::new()),
            internal_split_buf: Vec::new(),
            leaf_split_buf: Vec::new(),
            reinsert_buf: Vec::new(),
        }
    }

    pub fn insert(&mut self, data: T, data_aabb: Aabb<f64>) {
        if let InsertResult::Split(new_node) = self.root.insert(
            data,
            data_aabb,
            &mut self.internal_split_buf,
            &mut self.leaf_split_buf,
        ) {
            let root_aabb = self.root.bounds();
            let new_node_aabb = new_node.bounds();

            let old_root = mem::replace(&mut self.root, Node::Internal(ArrayVec::new()));

            match &mut self.root {
                Node::Internal(children) => {
                    children.push((Box::new(old_root), root_aabb));
                    children.push((new_node, new_node_aabb));
                }
                Node::Leaf(_) => unreachable!(),
            }
        }
    }

    pub fn retain(
        &mut self,
        mut collides: impl FnMut(Aabb<f64>) -> bool,
        mut retain: impl FnMut(&mut T, &mut Aabb<f64>) -> bool,
    ) {
        self.root
            .retain(None, &mut collides, &mut retain, &mut self.reinsert_buf);

        if let Node::Internal(children) = &mut self.root {
            if children.len() == 1 {
                let new_root = *children.drain(..).next().unwrap().0;
                self.root = new_root;
            } else if children.is_empty() {
                self.root = Node::Leaf(ArrayVec::new());
            }
        }

        let mut reinsert_buf = mem::take(&mut self.reinsert_buf);

        for (data, data_aabb) in reinsert_buf.drain(..) {
            self.insert(data, data_aabb);
        }

        debug_assert!(self.reinsert_buf.capacity() == 0);
        self.reinsert_buf = reinsert_buf;

        // Don't waste too much memory after a large restructuring.
        self.reinsert_buf.shrink_to(16);
    }

    pub fn query(
        &self,
        mut collides: impl FnMut(Aabb<f64>) -> bool,
        mut callback: impl FnMut(&T, Aabb<f64>) -> QueryAction,
    ) {
        self.root.query(&mut collides, &mut callback);
    }

    pub fn clear(&mut self) {
        self.root = Node::Leaf(ArrayVec::new());
    }

    pub fn depth(&self) -> usize {
        self.root.depth(0)
    }

    /// For the purposes of rendering the R-Tree.
    pub fn visit(&self, mut f: impl FnMut(Aabb<f64>, usize)) {
        self.root.visit(&mut f, 1);
        if self.root.children_count() != 0 {
            f(self.root.bounds(), 0);
        }
    }

    #[cfg(test)]
    fn check_invariants(&self, expected_len: usize) {
        assert!(self.internal_split_buf.is_empty());
        assert!(self.leaf_split_buf.is_empty());
        assert!(self.reinsert_buf.is_empty());

        if let Node::Internal(children) = &self.root {
            assert!(
                children.len() != 1,
                "internal root with a single entry should become the child"
            );
            assert!(!children.is_empty(), "empty internal root should be a leaf");
        }

        let mut len_counter = 0;

        self.root.check_invariants(None, 0, &mut len_counter);

        assert_eq!(
            len_counter, expected_len,
            "unexpected number of entries in rtree"
        )
    }
}

impl<T, const MIN: usize, const MAX: usize> Node<T, MIN, MAX> {
    fn bounds(&self) -> Aabb<f64> {
        match self {
            Node::Internal(children) => children
                .iter()
                .map(|(_, aabb)| *aabb)
                .reduce(Aabb::union)
                .unwrap(),
            Node::Leaf(children) => children
                .iter()
                .map(|(_, aabb)| *aabb)
                .reduce(Aabb::union)
                .unwrap(),
        }
    }

    fn children_count(&self) -> usize {
        match self {
            Node::Internal(children) => children.len(),
            Node::Leaf(children) => children.len(),
        }
    }

    fn insert(
        &mut self,
        data: T,
        data_aabb: Aabb<f64>,
        internal_split_buf: &mut InternalBuf<T, MIN, MAX>,
        leaf_split_buf: &mut LeafBuf<T>,
    ) -> InsertResult<T, MIN, MAX> {
        match self {
            Self::Internal(children) => {
                let children_is_full = children.is_full();

                let (best_child, best_child_aabb) = {
                    let best = area_insertion_heuristic(data_aabb, children);
                    &mut children[best]
                };

                match best_child.insert(data, data_aabb, internal_split_buf, leaf_split_buf) {
                    InsertResult::Ok => {
                        best_child_aabb.expand_to_contain(data_aabb);
                        InsertResult::Ok
                    }
                    InsertResult::Split(new_node) => {
                        let new_node_aabb = new_node.bounds();
                        *best_child_aabb = best_child.bounds();

                        if children_is_full {
                            let other = split_node::<_, MIN, MAX>(
                                internal_split_buf,
                                children,
                                (new_node, new_node_aabb),
                            );
                            InsertResult::Split(Box::new(Node::Internal(other)))
                        } else {
                            children.push((new_node, new_node_aabb));
                            InsertResult::Ok
                        }
                    }
                }
            }
            Self::Leaf(children) => {
                if children.is_full() {
                    let other =
                        split_node::<_, MIN, MAX>(leaf_split_buf, children, (data, data_aabb));
                    debug_assert!(other.len() >= MIN);

                    InsertResult::Split(Box::new(Node::Leaf(other)))
                } else {
                    children.push((data, data_aabb));
                    InsertResult::Ok
                }
            }
        }
    }

    fn query(
        &self,
        collides: &mut impl FnMut(Aabb<f64>) -> bool,
        callback: &mut impl FnMut(&T, Aabb<f64>) -> QueryAction,
    ) -> QueryAction {
        match self {
            Node::Internal(children) => {
                for child in children {
                    if collides(child.1) {
                        if let QueryAction::Break = child.0.query(collides, callback) {
                            return QueryAction::Break;
                        }
                    }
                }
            }
            Node::Leaf(children) => {
                for (child, child_aabb) in children {
                    if collides(*child_aabb) {
                        if let QueryAction::Break = callback(child, *child_aabb) {
                            return QueryAction::Break;
                        }
                    }
                }
            }
        }
        QueryAction::Continue
    }

    fn retain(
        &mut self,
        bounds: Option<Aabb<f64>>, // `None` when self is root.
        collides: &mut impl FnMut(Aabb<f64>) -> bool,
        retain: &mut impl FnMut(&mut T, &mut Aabb<f64>) -> bool,
        reinsert_buf: &mut LeafBuf<T>,
    ) -> RetainResult {
        match self {
            Node::Internal(children) => {
                let mut recalculate_bounds = false;

                children.retain(|(child, child_aabb)| {
                    if collides(*child_aabb) {
                        match child.retain(Some(*child_aabb), collides, retain, reinsert_buf) {
                            RetainResult::Ok => true,
                            RetainResult::Deleted => {
                                recalculate_bounds = true;
                                false
                            }
                            RetainResult::ShrunkAabb(new_aabb) => {
                                *child_aabb = new_aabb;
                                recalculate_bounds = true;
                                true
                            }
                        }
                    } else {
                        true
                    }
                });

                if let Some(bounds) = bounds {
                    if children.len() < MIN {
                        for (child, _) in children.drain(..) {
                            child.collect_orphans(reinsert_buf);
                        }
                        RetainResult::Deleted
                    } else if recalculate_bounds {
                        let new_bounds = self.bounds();
                        debug_assert!(bounds.contains_aabb(new_bounds));

                        if bounds != new_bounds {
                            RetainResult::ShrunkAabb(new_bounds)
                        } else {
                            RetainResult::Ok
                        }
                    } else {
                        RetainResult::Ok
                    }
                } else {
                    RetainResult::Ok
                }
            }
            Node::Leaf(children) => {
                let mut recalculate_bounds = false;

                let mut i = 0;
                while i < children.len() {
                    let (child, child_aabb) = &mut children[i];
                    let before = *child_aabb;
                    if collides(before) {
                        if retain(child, child_aabb) {
                            let after = *child_aabb;
                            if before != after {
                                if let Some(bounds) = bounds {
                                    recalculate_bounds = true;
                                    // A child can move within a leaf node without reinsertion
                                    // as long as it does not increase the bounds of the leaf.
                                    if !bounds.contains_aabb(after) {
                                        reinsert_buf.push(children.swap_remove(i));
                                    } else {
                                        i += 1;
                                    }
                                } else {
                                    i += 1;
                                }
                            } else {
                                i += 1;
                            }
                        } else {
                            recalculate_bounds = true;
                            children.swap_remove(i);
                        }
                    } else {
                        i += 1;
                    }
                }

                if let Some(bounds) = bounds {
                    if children.len() < MIN {
                        reinsert_buf.extend(children.drain(..));
                        RetainResult::Deleted
                    } else if recalculate_bounds {
                        let new_bounds = self.bounds();
                        debug_assert!(bounds.contains_aabb(new_bounds));

                        if bounds != new_bounds {
                            RetainResult::ShrunkAabb(new_bounds)
                        } else {
                            RetainResult::Ok
                        }
                    } else {
                        RetainResult::Ok
                    }
                } else {
                    RetainResult::Ok
                }
            }
        }
    }

    fn collect_orphans(self, reinsert_buf: &mut LeafBuf<T>) {
        match self {
            Node::Internal(children) => {
                for (child, _) in children {
                    child.collect_orphans(reinsert_buf);
                }
            }
            Node::Leaf(children) => reinsert_buf.extend(children),
        }
    }

    fn depth(&self, level: usize) -> usize {
        match self {
            Node::Internal(children) => children[0].0.depth(level + 1),
            Node::Leaf(_) => level,
        }
    }

    fn visit(&self, f: &mut impl FnMut(Aabb<f64>, usize), level: usize) {
        match self {
            Node::Internal(children) => {
                for (child, child_aabb) in children {
                    child.visit(f, level + 1);
                    f(*child_aabb, level);
                }
            }
            Node::Leaf(children) => {
                for (_, child_aabb) in children {
                    f(*child_aabb, level);
                }
            }
        }
    }

    #[cfg(test)]
    fn check_invariants(
        &self,
        bounds: Option<Aabb<f64>>,
        depth: usize,
        len_counter: &mut usize,
    ) -> usize {
        let mut child_depth = None;

        match self {
            Node::Internal(children) => {
                assert!(!children.is_empty());

                if let Some(bounds) = bounds {
                    let tight_bounds = self.bounds();
                    assert!(
                        relative_eq!(tight_bounds.min, bounds.min)
                            && relative_eq!(tight_bounds.max, bounds.max),
                        "bounding rectangle for internal node is not tight"
                    );
                }

                for (child, child_aabb) in children {
                    let d = child.check_invariants(Some(*child_aabb), depth + 1, len_counter);
                    if let Some(child_depth) = &mut child_depth {
                        assert_eq!(*child_depth, d, "rtree is not balanced");
                    } else {
                        child_depth = Some(d);
                    }
                }
            }
            Node::Leaf(children) => {
                if let Some(bounds) = bounds {
                    let tight_bounds = self.bounds();
                    assert!(
                        relative_eq!(tight_bounds.min, bounds.min)
                            && relative_eq!(tight_bounds.max, bounds.max),
                        "bounding rectangle for leaf node is not tight"
                    );
                }

                *len_counter += children.len();
                child_depth = Some(depth);
            }
        }

        if let Some(bounds) = bounds {
            assert!(bounds == self.bounds());
        }

        child_depth.unwrap()
    }
}
enum InsertResult<T, const MIN: usize, const MAX: usize> {
    /// No split occurred.
    Ok,
    /// Contains the new node that was split off.
    Split(Box<Node<T, MIN, MAX>>),
}

enum RetainResult {
    /// Nothing changed.
    Ok,
    /// This node must be deleted from its parent.
    Deleted,
    /// This node was not deleted but its AABR was shrunk.
    /// Contains the new AABR.
    ShrunkAabb(Aabb<f64>),
}

fn area_insertion_heuristic<T>(data_aabb: Aabb<f64>, children: &[(T, Aabb<f64>)]) -> usize {
    debug_assert!(
        !children.is_empty(),
        "internal node must have at least one child"
    );

    let mut best = 0;
    let mut best_area_increase = f64::INFINITY;
    let mut best_aabb = Aabb::default();

    for (idx, (_, child_aabb)) in children.iter().enumerate() {
        let area_increase = volume(child_aabb.union(data_aabb)) - volume(*child_aabb);
        if area_increase < best_area_increase {
            best = idx;
            best_area_increase = area_increase;
            best_aabb = *child_aabb;
        } else if area_increase == best_area_increase && volume(*child_aabb) < volume(best_aabb) {
            best = idx;
            best_aabb = *child_aabb;
        }
    }

    best
}

/// Splits a node with `children` being the children of the node being split.
///
/// After returning, `children` contains half the data while the returned
/// `ArrayVec` contains the other half for the new node.
fn split_node<T, const MIN: usize, const MAX: usize>(
    split_buf: &mut Vec<(T, Aabb<f64>)>,
    children: &mut ArrayVec<(T, Aabb<f64>), MAX>,
    data: (T, Aabb<f64>),
) -> ArrayVec<(T, Aabb<f64>), MAX> {
    split_buf.extend(children.take());
    split_buf.push(data);

    let dists = MIN..MAX - MIN + 2;

    let bb = |es: &[(T, Aabb<f64>)]| es.iter().map(|e| e.1).reduce(Aabb::union).unwrap();

    let mut sum_x = 0.0;
    split_buf.sort_unstable_by_key(|e| OrderedFloat(e.1.min.x / 2.0 + e.1.max.x / 2.0));

    for split in dists.clone() {
        sum_x += surface_area(bb(&split_buf[..split])) + surface_area(bb(&split_buf[split..]));
    }

    let mut sum_y = 0.0;
    split_buf.sort_unstable_by_key(|e| OrderedFloat(e.1.min.y / 2.0 + e.1.max.y / 2.0));

    for split in dists.clone() {
        sum_y += surface_area(bb(&split_buf[..split])) + surface_area(bb(&split_buf[split..]));
    }

    let mut sum_z = 0.0;
    split_buf.sort_unstable_by_key(|e| OrderedFloat(e.1.min.z / 2.0 + e.1.max.z / 2.0));

    for split in dists.clone() {
        sum_z += surface_area(bb(&split_buf[..split])) + surface_area(bb(&split_buf[split..]));
    }

    // Sort by the winning axis
    split_buf.sort_unstable_by_key(|e| {
        let (min, max) = if sum_x <= sum_y && sum_x <= sum_z {
            (e.1.min.x, e.1.max.x)
        } else if sum_y <= sum_x && sum_y <= sum_z {
            (e.1.min.y, e.1.max.y)
        } else {
            (e.1.min.z, e.1.max.z)
        };

        OrderedFloat(min / 2.0 + max / 2.0)
    });

    let mut best_dist = 0;
    let mut best_overlap_value = f64::INFINITY;
    let mut best_area_value = f64::INFINITY;

    for split in dists {
        let group_1 = bb(&split_buf[..split]);
        let group_2 = bb(&split_buf[split..]);
        let overlap_value = {
            let int = group_1.intersection(group_2);
            if int.is_valid() {
                volume(int)
            } else {
                0.0
            }
        };
        let area_value = volume(group_1) + volume(group_2);

        if overlap_value < best_overlap_value {
            best_overlap_value = overlap_value;
            best_area_value = area_value;
            best_dist = split;
        } else if overlap_value == best_overlap_value && area_value < best_area_value {
            best_area_value = area_value;
            best_dist = split;
        }
    }

    debug_assert!(children.is_empty());
    debug_assert_eq!(split_buf.len(), MAX + 1);

    let mut other = ArrayVec::new();
    other.extend(split_buf.drain(best_dist..));

    children.extend(split_buf.drain(..));

    other
}

fn volume(aabb: Aabb<f64>) -> f64 {
    (aabb.max - aabb.min).product()
}

fn surface_area(aabb: Aabb<f64>) -> f64 {
    let d = aabb.max - aabb.min;
    (d.x * d.y + d.x * d.z + d.y * d.z) * 2.0
}

#[cfg(test)]
mod tests {
    use std::f64::consts::TAU;
    use std::sync::atomic::{AtomicU64, Ordering};

    use rand::Rng;
    use vek::Vec3;

    use super::*;

    fn insert_rand<const MIN: usize, const MAX: usize>(
        rtree: &mut RTree<u64, MIN, MAX>,
    ) -> (u64, Aabb<f64>) {
        static NEXT_UNIQUE_ID: AtomicU64 = AtomicU64::new(0);

        let id = NEXT_UNIQUE_ID.fetch_add(1, Ordering::SeqCst);

        let mut rng = rand::thread_rng();

        let min = Vec3::new(rng.gen(), rng.gen(), rng.gen());
        let max = Vec3::new(
            min.x + rng.gen_range(0.003..=0.01),
            min.y + rng.gen_range(0.003..=0.01),
            min.z + rng.gen_range(0.003..=0.01),
        );

        let aabb = Aabb { min, max };

        rtree.insert(id, aabb);

        (id, aabb)
    }

    #[test]
    fn insert_delete_interleaved() {
        let mut rtree: RTree<u64, 4, 8> = RTree::new();

        for i in 0..5_000 {
            insert_rand(&mut rtree);
            let (id_0, aabb_0) = insert_rand(&mut rtree);

            let mut found = false;
            rtree.retain(
                |aabb| aabb.collides_with_aabb(aabb_0),
                |&mut id, _| {
                    if id == id_0 {
                        assert!(!found);
                        found = true;
                        false
                    } else {
                        true
                    }
                },
            );
            assert!(found);

            rtree.check_invariants(i + 1);
        }
    }

    #[test]
    fn node_underfill() {
        let mut rtree: RTree<u64, 4, 8> = RTree::new();

        for i in 0..5_000 {
            insert_rand(&mut rtree);
            rtree.check_invariants(i + 1);
        }

        let mut delete_count = 0;

        rtree.retain(
            |_| true,
            |_, _| {
                if rand::random() {
                    delete_count += 1;
                    false
                } else {
                    true
                }
            },
        );
        rtree.check_invariants(5_000 - delete_count);

        rtree.clear();
        rtree.check_invariants(0);
    }

    #[test]
    fn movement() {
        let mut rtree: RTree<u64, 4, 8> = RTree::new();

        for _ in 0..5_000 {
            insert_rand(&mut rtree);
        }

        let mut rng = rand::thread_rng();
        for _ in 0..100 {
            rtree.retain(
                |_| true,
                |_, aabb| {
                    let angle = rng.gen_range(0.0..TAU);
                    let z: f64 = rng.gen_range(-1.0..=1.0);

                    let v = Vec3::new(
                        (1.0 - z * z).sqrt() * angle.cos(),
                        (1.0 - z * z).sqrt() * angle.sin(),
                        z,
                    ) * 0.03;

                    aabb.min += v;
                    aabb.max += v;
                    assert!(aabb.is_valid());

                    true
                },
            );
            rtree.check_invariants(5_000);
        }
    }
}
