use vek::{Aabb, Vec3};

pub mod bvh;

pub trait SpatialIndex<N = f64> {
    type Object: Bounded3D<N>;

    /// Invokes `f` with every object in the spatial index considered
    /// colliding according to `collides` in an arbitrary order.
    ///
    /// `collides` takes an AABB and returns whether or not a collision
    /// occurred with the given AABB.
    ///
    /// `f` is called with every object considered colliding. If `f` returns
    /// with `Some(x)`, then `query` exits early with `Some(x)`. If `f` never
    /// returns with `Some`, then query returns `None`.
    fn query<C, F, T>(&self, collides: C, f: F) -> Option<T>
    where
        C: FnMut(Aabb<N>) -> bool,
        F: FnMut(&Self::Object) -> Option<T>;

    /// Casts a ray defined by `origin` and `direction` through object AABBs
    /// and returns the closest intersection for which `f` returns `true`.
    ///
    /// `f` is a predicate used to filter intersections. For instance, if a ray
    /// is shot from a player's eye position, you probably don't want the
    /// ray to intersect with the player's own hitbox.
    ///
    /// If no intersections are found or if `f` never returns `true` then `None`
    /// is returned. Additionally, the given ray direction must be
    /// normalized.
    fn raycast<F>(
        &self,
        origin: Vec3<f64>,
        direction: Vec3<f64>,
        f: F,
    ) -> Option<RaycastHit<Self::Object, N>>
    where
        F: FnMut(RaycastHit<Self::Object, N>) -> bool;
}

pub trait Bounded3D<N = f64> {
    fn aabb(&self) -> Aabb<N>;
}

/// Represents an intersection between a ray and an entity's axis-aligned
/// bounding box (hitbox).
#[derive(PartialEq, Eq, Debug)]
pub struct RaycastHit<'a, O, N = f64> {
    /// The object that was hit by the ray.
    pub object: &'a O,
    /// The distance from the ray origin to the closest intersection point.
    /// If the origin of the ray is inside the bounding box, then this will be
    /// zero.
    pub near: N,
    /// The distance from the ray origin to the second intersection point. This
    /// represents the point at which the ray exits the bounding box.
    pub far: N,
}

impl<O, N: Clone> Clone for RaycastHit<'_, O, N> {
    fn clone(&self) -> Self {
        Self {
            object: self.object,
            near: self.near.clone(),
            far: self.far.clone(),
        }
    }
}

impl<O, N: Copy> Copy for RaycastHit<'_, O, N> {}

impl<N: Clone> Bounded3D<N> for Aabb<N> {
    fn aabb(&self) -> Aabb<N> {
        self.clone()
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash, Default, Debug)]
pub struct WithAabb<O, N = f64> {
    pub object: O,
    pub aabb: Aabb<N>,
}

impl<O, N: Clone> Bounded3D<N> for WithAabb<O, N> {
    fn aabb(&self) -> Aabb<N> {
        self.aabb.clone()
    }
}

/// Calculates the intersection between an axis-aligned bounding box and a ray
/// defined by its origin `ro` and direction `rd`.
///
/// If an intersection occurs, `Some((near, far))` is returned. `near` and `far`
/// are the distance from the origin to the closest and furthest intersection
/// points respectively. If the intersection occurs inside the bounding box,
/// then `near` is zero.
pub fn ray_box_intersect(ro: Vec3<f64>, rd: Vec3<f64>, bb: Aabb<f64>) -> Option<(f64, f64)> {
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
                if let Some((near, far)) = ray_box_intersect(ro, rd, bb) {
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