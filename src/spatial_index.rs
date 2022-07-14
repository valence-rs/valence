//! Efficient spatial entity queries.

use std::iter::FusedIterator;

use rayon::iter::{IndexedParallelIterator, ParallelIterator};
use vek::{Aabb, Vec3};

use crate::bvh::{Bvh, Node};
use crate::entity::{Entities, EntityId};
use crate::util::ray_box_intersect;
use crate::world::WorldId;

/// A data structure for fast spatial queries on entity [hitboxes]. This is used
/// to accelerate tasks such as collision detection and ray tracing.
///
/// The spatial index is only updated at the end of each tick. Any modification
/// to an entity that would change its hitbox is not reflected in the spatial
/// index until the end of the tick.
///
/// [hitboxes]: crate::entity::Entity::hitbox
pub struct SpatialIndex {
    bvh: Bvh<EntityId>,
}

impl SpatialIndex {
    pub(crate) fn new() -> Self {
        Self { bvh: Bvh::new() }
    }

    #[doc(hidden)]
    #[deprecated = "This is for documentation tests only!"]
    pub fn example_new() -> Self {
        dbg!("Don't call me from outside tests!");
        Self::new()
    }

    /// Invokes `f` with every entity in the spatial index considered
    /// colliding according to `collides`.
    ///
    /// `collides` takes an AABB and returns whether or not a collision
    /// occurred with the given AABB.
    ///
    /// `f` is called with the entity ID and hitbox of all colliding entities.
    /// If `f` returns with `Some(x)`, then `query` exits early with
    /// `Some(x)`. If `f` never returns with `Some`, then query returns `None`.
    ///
    /// # Examples
    ///
    /// Visit all entities intersecting a 10x10x10 cube centered at the origin.
    ///
    /// ```
    /// # #[allow(deprecated)]
    /// # let si = valence::spatial_index::SpatialIndex::example_new();
    /// use valence::vek::*;
    ///
    /// let cube = Aabb {
    ///     min: Vec3::new(-5.0, -5.0, -5.0),
    ///     max: Vec3::new(5.0, 5.0, 5.0),
    /// };
    ///
    /// let collides = |aabb: Aabb<f64>| aabb.collides_with_aabb(cube);
    /// let f = |id, _| -> Option<()> {
    ///     println!("Found entity: {id:?}");
    ///     None
    /// };
    ///
    /// // Assume `si` is the spatial index
    /// si.query(collides, f);
    /// ```
    pub fn query<C, F, T>(&self, mut collides: C, mut f: F) -> Option<T>
    where
        C: FnMut(Aabb<f64>) -> bool,
        F: FnMut(EntityId, Aabb<f64>) -> Option<T>,
    {
        fn query_rec<C, F, T>(node: Node<EntityId>, collides: &mut C, f: &mut F) -> Option<T>
        where
            C: FnMut(Aabb<f64>) -> bool,
            F: FnMut(EntityId, Aabb<f64>) -> Option<T>,
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
                Node::Leaf { data, bb } => {
                    if collides(bb) {
                        f(*data, bb)
                    } else {
                        None
                    }
                }
            }
        }

        query_rec(self.bvh.traverse()?, &mut collides, &mut f)
    }

    /// Casts a ray defined by `origin` and `direction` through entity hitboxes
    /// and returns the closest intersection for which `f` returns `true`.
    ///
    /// `f` is a predicate which can be used to filter intersections. For
    /// instance, if a ray is shot from a player's eye position, you probably
    /// don't want the ray to intersect with the player's own hitbox.
    ///
    /// If no intersections are found or if `f` never returns `true` then `None`
    /// is returned. Additionally, the given ray direction must be
    /// normalized.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[allow(deprecated)]
    /// # let si = valence::spatial_index::SpatialIndex::example_new();
    /// use valence::vek::*;
    ///
    /// let origin = Vec3::new(0.0, 0.0, 0.0);
    /// let direction = Vec3::new(1.0, 1.0, 1.0).normalized();
    ///
    /// // Assume `si` is the spatial index.
    /// if let Some(hit) = si.raycast(origin, direction, |_| true) {
    ///     println!("Raycast hit! {hit:?}");
    /// }
    /// ```
    pub fn raycast<F>(
        &self,
        origin: Vec3<f64>,
        direction: Vec3<f64>,
        mut f: F,
    ) -> Option<RaycastHit>
    where
        F: FnMut(&RaycastHit) -> bool,
    {
        fn raycast_rec(
            node: Node<EntityId>,
            hit: &mut Option<RaycastHit>,
            near: f64,
            far: f64,
            origin: Vec3<f64>,
            direction: Vec3<f64>,
            f: &mut impl FnMut(&RaycastHit) -> bool,
        ) {
            if let Some(hit) = hit {
                if hit.near <= near {
                    return;
                }
            }

            match node {
                Node::Internal(int) => {
                    let (_, left, right) = int.split();

                    let int_left = ray_box_intersect(origin, direction, left.bb());
                    let int_right = ray_box_intersect(origin, direction, right.bb());

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
                Node::Leaf { data, bb } => {
                    let this_hit = RaycastHit {
                        entity: *data,
                        bb,
                        near,
                        far,
                    };

                    if f(&this_hit) {
                        *hit = Some(this_hit);
                    }
                }
            }
        }

        debug_assert!(
            direction.is_normalized(),
            "the ray direction must be normalized"
        );

        let root = self.bvh.traverse()?;
        let (near, far) = ray_box_intersect(origin, direction, root.bb())?;

        let mut hit = None;
        raycast_rec(root, &mut hit, near, far, origin, direction, &mut f);
        hit
    }

    /// Returns an iterator over all entities and their hitboxes in
    /// an unspecified order.
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (EntityId, Aabb<f64>)> + FusedIterator + Clone + '_ {
        self.bvh.iter().map(|(&id, bb)| (id, bb))
    }

    /// Returns a parallel iterator over all entities and their
    /// hitboxes in an unspecified order.
    pub fn par_iter(
        &self,
    ) -> impl IndexedParallelIterator<Item = (EntityId, Aabb<f64>)> + Clone + '_ {
        self.bvh.par_iter().map(|(&id, bb)| (id, bb))
    }

    pub(crate) fn update(&mut self, entities: &Entities, id: WorldId) {
        self.bvh.build(
            entities
                .iter()
                .filter(|(_, e)| e.world() == id)
                .map(|(id, e)| (id, e.hitbox())),
        )
    }
}

/// Represents an intersection between a ray and an entity's axis-aligned
/// bounding box (hitbox).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct RaycastHit {
    /// The [`EntityId`] of the entity that was hit by the ray.
    pub entity: EntityId,
    /// The bounding box (hitbox) of the entity that was hit.
    pub bb: Aabb<f64>,
    /// The distance from the ray origin to the closest intersection point.
    /// If the origin of the ray is inside the bounding box, then this will be
    /// zero.
    pub near: f64,
    /// The distance from the ray origin to the second intersection point. This
    /// represents the point at which the ray exits the bounding box.
    pub far: f64,
}
