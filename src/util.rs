//! Miscellaneous utilities.

use std::iter::FusedIterator;

use num::cast::AsPrimitive;
use num::Float;
use vek::{Aabb, Vec3};

use crate::chunk_pos::ChunkPos;

const EXTRA_RADIUS: i32 = 3;

/// Returns an iterator over all chunk positions within a view distance,
/// centered on a particular chunk position.
pub fn chunks_in_view_distance(
    center: ChunkPos,
    distance: u8,
) -> impl FusedIterator<Item = ChunkPos> {
    let dist = distance as i32 + EXTRA_RADIUS;
    (center.z - dist..=center.z + dist)
        .flat_map(move |z| (center.x - dist..=center.x + dist).map(move |x| ChunkPos { x, z }))
        .filter(move |&p| is_chunk_in_view_distance(center, p, distance))
}

/// Checks if two chunks are within a view distance of each other such that a
/// client standing in one chunk would be able to see the other.
pub fn is_chunk_in_view_distance(p0: ChunkPos, p1: ChunkPos, distance: u8) -> bool {
    (p0.x as f64 - p1.x as f64).powi(2) + (p0.z as f64 - p1.z as f64).powi(2)
        <= (distance as f64 + EXTRA_RADIUS as f64).powi(2)
}

pub(crate) fn aabb_from_bottom_and_size<T>(bottom: Vec3<T>, size: Vec3<T>) -> Aabb<T>
where
    T: Float + 'static,
    f64: AsPrimitive<T>,
{
    let aabb = Aabb {
        min: Vec3::new(
            bottom.x - size.x / 2.0.as_(),
            bottom.y,
            bottom.z - size.z / 2.0.as_(),
        ),
        max: Vec3::new(
            bottom.x + size.x / 2.0.as_(),
            bottom.y + size.y,
            bottom.z + size.z / 2.0.as_(),
        ),
    };

    debug_assert!(aabb.is_valid());

    aabb
}

/// Takes a normalized direction vector and returns a `(yaw, pitch)` tuple in
/// degrees.
///
/// This function is the inverse of [`from_yaw_and_pitch`] except for the case
/// where the direction is straight up or down.
pub fn to_yaw_and_pitch(d: Vec3<f64>) -> (f64, f64) {
    debug_assert!(d.is_normalized(), "the given vector should be normalized");

    let yaw = f64::atan2(d.z, d.x).to_degrees() - 90.0;
    let pitch = -(d.y).asin().to_degrees();
    (yaw, pitch)
}

/// Takes yaw and pitch angles (in degrees) and returns a normalized
/// direction vector.
///
/// This function is the inverse of [`to_yaw_and_pitch`].
pub fn from_yaw_and_pitch(yaw: f64, pitch: f64) -> Vec3<f64> {
    let yaw = (yaw + 90.0).to_radians();
    let pitch = (-pitch).to_radians();

    let xz_len = pitch.cos();
    Vec3::new(yaw.cos() * xz_len, pitch.sin(), yaw.sin() * xz_len)
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

/// Calculates the minimum number of bits needed to represent the integer `n`.
/// Also known as `floor(log2(n)) + 1`.
///
/// This returns `0` if `n` is `0`.
pub(crate) const fn bits_needed(n: usize) -> usize {
    (usize::BITS - n.leading_zeros()) as _
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use rand::random;

    use super::*;

    #[test]
    fn yaw_pitch_round_trip() {
        for _ in 0..=100 {
            let d = (Vec3::new(random(), random(), random()) * 2.0 - 1.0).normalized();

            let (yaw, pitch) = to_yaw_and_pitch(d);
            let d_new = from_yaw_and_pitch(yaw, pitch);

            assert_relative_eq!(d, d_new, epsilon = f64::EPSILON * 100.0);
        }
    }

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
