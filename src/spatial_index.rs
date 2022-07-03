use vek::{Aabb, Vec3};

use crate::bvh::Bvh;
pub use crate::bvh::TraverseStep;
use crate::{Entities, EntityId, WorldId};

pub struct SpatialIndex {
    bvh: Bvh<EntityId>,
}

impl SpatialIndex {
    pub(crate) fn new() -> Self {
        Self { bvh: Bvh::new() }
    }

    pub fn traverse<F, T>(&self, mut f: F) -> Option<T>
    where
        F: FnMut(Option<EntityId>, Aabb<f64>) -> TraverseStep<T>,
    {
        self.bvh.traverse(|e, bb| f(e.cloned(), bb))
    }

    pub fn query<C, F, T>(&self, mut collides: C, mut f: F) -> Option<T>
    where
        C: FnMut(Aabb<f64>) -> bool,
        F: FnMut(EntityId, Aabb<f64>) -> Option<T>,
    {
        self.traverse(|e, bb| {
            if collides(bb) {
                e.and_then(|id| f(id, bb))
                    .map_or(TraverseStep::Hit, TraverseStep::Return)
            } else {
                TraverseStep::Miss
            }
        })
    }

    // TODO: accept predicate here. Might want to skip invisible entities, for
    // instance.
    pub fn raycast(&self, origin: Vec3<f64>, direction: Vec3<f64>) -> Option<RaycastHit> {
        debug_assert!(
            direction.is_normalized(),
            "the ray direction must be normalized"
        );

        let mut hit: Option<RaycastHit> = None;

        self.traverse::<_, ()>(|entity, bb| {
            if let Some((near, far)) = ray_box_intersection(origin, direction, bb) {
                if hit.as_ref().map_or(true, |hit| near < hit.near) {
                    if let Some(entity) = entity {
                        hit = Some(RaycastHit {
                            entity,
                            bb,
                            near,
                            far,
                        });
                    }
                    TraverseStep::Hit
                } else {
                    // Do not explore subtrees that cannot produce an intersection closer than the
                    // closest we've seen so far.
                    TraverseStep::Miss
                }
            } else {
                TraverseStep::Miss
            }
        });

        hit
    }

    pub fn raycast_all<F, T>(&self, origin: Vec3<f64>, direction: Vec3<f64>, mut f: F) -> Option<T>
    where
        F: FnMut(RaycastHit) -> Option<T>,
    {
        debug_assert!(
            direction.is_normalized(),
            "the ray direction must be normalized"
        );

        self.traverse(
            |entity, bb| match (ray_box_intersection(origin, direction, bb), entity) {
                (Some((near, far)), Some(entity)) => {
                    let hit = RaycastHit {
                        entity,
                        bb,
                        near,
                        far,
                    };
                    f(hit).map_or(TraverseStep::Hit, TraverseStep::Return)
                }
                (Some(_), None) => TraverseStep::Hit,
                (None, _) => TraverseStep::Miss,
            },
        )
    }

    pub(crate) fn update(&mut self, entities: &Entities, id: WorldId) {
        self.bvh.build(
            entities
                .iter()
                .filter(|(_, e)| e.world() == Some(id))
                .map(|(id, e)| (id, e.hitbox())),
        )
    }
}

/// Represents an intersection between a ray and an entity's axis-aligned
/// bounding box.
#[derive(Clone, Copy, PartialEq)]
pub struct RaycastHit {
    /// The [`EntityId`] of the entity that was hit by the ray.
    pub entity: EntityId,
    /// The bounding box of the entity that was hit.
    pub bb: Aabb<f64>,
    /// The distance from the ray origin to the closest intersection point.
    /// If the origin of the ray is inside the bounding box, then this will be
    /// zero.
    pub near: f64,
    /// The distance from the ray origin to the second intersection point. This
    /// represents the point at which the ray exits the bounding box.
    pub far: f64,
}

fn ray_box_intersection(ro: Vec3<f64>, rd: Vec3<f64>, bb: Aabb<f64>) -> Option<(f64, f64)> {
    let mut near = -f64::INFINITY;
    let mut far = f64::INFINITY;

    for i in 0..3 {
        // Rust's definition of min and max properly handle the NaNs that these
        // computations might produce.
        let t0 = (bb.min[i] - ro[i]) / rd[i];
        let t1 = (bb.max[i] - ro[i]) / rd[i];

        near = near.max(t0.min(t1));
        far = far.min(t0.max(t1));
    }

    if near <= far && far >= 0.0 {
        Some((near.max(0.0), far))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_box_edge_cases() {
        let bb = Aabb {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(1.0, 1.0, 1.0),
        };

        let ros = [
            // On a corner
            Vec3::new(0.0, 0.0, 0.0),
            // Outside
            Vec3::new(-0.5, 0.5, -0.5),
            // In the center
            Vec3::new(0.5, 0.5, 0.5),
            // On an edge
            Vec3::new(0.0, 0.5, 0.0),
            // On a face
            Vec3::new(0.0, 0.5, 0.5),
            // Outside slabs
            Vec3::new(-2.0, -2.0, -2.0),
        ];

        let rds = [
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 0.0, -1.0),
        ];

        assert!(rds.iter().all(|d| d.is_normalized()));

        for ro in ros {
            for rd in rds {
                if let Some((near, far)) = ray_box_intersection(ro, rd, bb) {
                    assert!(near.is_finite());
                    assert!(far.is_finite());
                    assert!(near <= far);
                    assert!(near >= 0.0);
                    assert!(far >= 0.0);
                }
            }
        }
    }
}
