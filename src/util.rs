//! Miscellaneous utilities.

use num::cast::AsPrimitive;
use num::Float;
use vek::{Aabb, Vec3};

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

/// Calculates the minimum number of bits needed to represent the integer `n`.
/// Also known as `floor(log2(n)) + 1`.
///
/// This returns `0` if `n` is `0`.
pub(crate) const fn bit_width(n: usize) -> usize {
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
}
