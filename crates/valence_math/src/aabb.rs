use std::ops::{Add, Sub};

use glam::DVec3;

/// A three-dimensional axis-aligned bounding box, or "AABB".
///
/// The AABB is defined by two points—`min` and `max`. `min` is less than or
/// equal to `max` componentwise.
#[derive(Copy, Clone, PartialEq, Default, Debug)]
pub struct Aabb {
    min: DVec3,
    max: DVec3,
}

impl Aabb {
    pub const ZERO: Self = Self {
        min: DVec3::ZERO,
        max: DVec3::ZERO,
    };

    /// Constructs a new AABB from `min` and `max` points.
    ///
    /// # Panics
    ///
    /// Panics if `debug_assertions` are enabled and `min` is not less than or
    /// equal to `max` componentwise.
    #[cfg_attr(debug_assertions, track_caller)]
    pub fn new(min: DVec3, max: DVec3) -> Self {
        debug_assert!(
            min.x <= max.x && min.y <= max.y && min.z <= max.z,
            "`min` must be less than or equal to `max` componentwise (min = {min}, max = {max})"
        );

        Self { min, max }
    }

    // TODO: remove when the assertion in `new` can be done in a `const` context.
    #[doc(hidden)]
    pub const fn new_unchecked(min: DVec3, max: DVec3) -> Self {
        Self { min, max }
    }

    /// Returns a new AABB containing a single point `p`.
    pub fn new_point(p: DVec3) -> Self {
        Self::new(p, p)
    }

    pub fn from_bottom_size(bottom: DVec3, size: DVec3) -> Self {
        Self::new(
            DVec3 {
                x: bottom.x - size.x / 2.0,
                y: bottom.y,
                z: bottom.z - size.z / 2.0,
            },
            DVec3 {
                x: bottom.x + size.x / 2.0,
                y: bottom.y + size.y,
                z: bottom.z + size.z / 2.0,
            },
        )
    }

    pub const fn min(self) -> DVec3 {
        self.min
    }

    pub const fn max(self) -> DVec3 {
        self.max
    }

    pub fn union(self, other: Self) -> Self {
        Self::new(self.min.min(other.min), self.max.max(other.max))
    }

    pub fn intersects(self, other: Self) -> bool {
        self.max.x >= other.min.x
            && other.max.x >= self.min.x
            && self.max.y >= other.min.y
            && other.max.y >= self.min.y
            && self.max.z >= other.min.z
            && other.max.z >= self.min.z
    }

    /// Does this bounding box contain the given point?
    pub fn contains_point(self, p: DVec3) -> bool {
        self.min.x <= p.x
            && self.min.y <= p.y
            && self.min.z <= p.z
            && self.max.x >= p.x
            && self.max.y >= p.y
            && self.max.z >= p.z
    }

    /// Returns the closest point in the AABB to the given point.
    pub fn projected_point(self, p: DVec3) -> DVec3 {
        p.clamp(self.min, self.max)
    }

    /// Returns the smallest distance from the AABB to the point.
    pub fn distance_to_point(self, p: DVec3) -> f64 {
        self.projected_point(p).distance(p)
    }

    /// Calculates the intersection between this AABB and a ray
    /// defined by its `origin` point and `direction` vector.
    ///
    /// If an intersection occurs, `Some([near, far])` is returned. `near` and
    /// `far` are the values of `t` in the equation `origin + t * direction =
    /// point` where `point` is the nearest or furthest intersection point to
    /// the `origin`. If no intersection occurs, then `None` is returned.
    ///
    /// In other words, if `direction` is normalized, then `near` and `far` are
    /// the distances to the nearest and furthest intersection points.
    pub fn ray_intersection(self, origin: DVec3, direction: DVec3) -> Option<[f64; 2]> {
        let mut near: f64 = 0.0;
        let mut far = f64::INFINITY;

        for i in 0..3 {
            // Rust's definition of `min` and `max` properly handle the NaNs these
            // computations may produce.
            let t0 = (self.min[i] - origin[i]) / direction[i];
            let t1 = (self.max[i] - origin[i]) / direction[i];

            near = near.max(t0.min(t1));
            far = far.min(t0.max(t1));
        }

        if near <= far {
            Some([near, far])
        } else {
            None
        }
    }
}

impl Add<DVec3> for Aabb {
    type Output = Aabb;

    fn add(self, rhs: DVec3) -> Self::Output {
        Self::new(self.min + rhs, self.max + rhs)
    }
}

impl Add<Aabb> for DVec3 {
    type Output = Aabb;

    fn add(self, rhs: Aabb) -> Self::Output {
        rhs + self
    }
}

impl Sub<DVec3> for Aabb {
    type Output = Aabb;

    fn sub(self, rhs: DVec3) -> Self::Output {
        Self::new(self.min - rhs, self.max - rhs)
    }
}

impl Sub<Aabb> for DVec3 {
    type Output = Aabb;

    fn sub(self, rhs: Aabb) -> Self::Output {
        rhs - self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_intersect_edge_cases() {
        let bb = Aabb::new([0.0, 0.0, 0.0].into(), [1.0, 1.0, 1.0].into());

        let ros = [
            // On a corner
            DVec3::new(0.0, 0.0, 0.0),
            // Outside
            DVec3::new(-0.5, 0.5, -0.5),
            // In the center
            DVec3::new(0.5, 0.5, 0.5),
            // On an edge
            DVec3::new(0.0, 0.5, 0.0),
            // On a face
            DVec3::new(0.0, 0.5, 0.5),
            // Outside slabs
            DVec3::new(-2.0, -2.0, -2.0),
        ];

        let rds = [
            DVec3::new(1.0, 0.0, 0.0),
            DVec3::new(-1.0, 0.0, 0.0),
            DVec3::new(0.0, 1.0, 0.0),
            DVec3::new(0.0, -1.0, 0.0),
            DVec3::new(0.0, 0.0, 1.0),
            DVec3::new(0.0, 0.0, -1.0),
        ];

        assert!(rds.iter().all(|d| d.is_normalized()));

        for ro in ros {
            for rd in rds {
                if let Some([near, far]) = bb.ray_intersection(ro, rd) {
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
