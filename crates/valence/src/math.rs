pub use glam::*;

/// An axis-aligned bounding box. `min` is expected to be <= `max`
/// componentwise.
#[derive(Copy, Clone, PartialEq, Default, Debug)]
pub struct Aabb {
    pub min: DVec3,
    pub max: DVec3,
}

impl Aabb {
    pub fn new(p0: impl Into<DVec3>, p1: impl Into<DVec3>) -> Self {
        let p0 = p0.into();
        let p1 = p1.into();
        Self {
            min: p0.min(p1),
            max: p0.max(p1),
        }
    }

    pub(crate) fn from_bottom_size(bottom: impl Into<DVec3>, size: impl Into<DVec3>) -> Self {
        let bottom = bottom.into();
        let size = size.into();

        Self {
            min: DVec3 {
                x: bottom.x - size.x / 2.0,
                y: bottom.y,
                z: bottom.z - size.z / 2.0,
            },
            max: DVec3 {
                x: bottom.x + size.x / 2.0,
                y: bottom.y + size.y,
                z: bottom.z + size.z / 2.0,
            },
        }
    }
}

/// Takes a normalized direction vector and returns a `(yaw, pitch)` tuple in
/// degrees.
///
/// This function is the inverse of [`from_yaw_and_pitch`] except for the case
/// where the direction is straight up or down.
#[track_caller]
pub fn to_yaw_and_pitch(d: Vec3) -> (f32, f32) {
    debug_assert!(d.is_normalized(), "the given vector should be normalized");

    let yaw = f32::atan2(d.z, d.x).to_degrees() - 90.0;
    let pitch = -(d.y).asin().to_degrees();
    (yaw, pitch)
}

/// Takes yaw and pitch angles (in degrees) and returns a normalized
/// direction vector.
///
/// This function is the inverse of [`to_yaw_and_pitch`].
pub fn from_yaw_and_pitch(yaw: f32, pitch: f32) -> Vec3 {
    let (yaw_sin, yaw_cos) = (yaw + 90.0).to_radians().sin_cos();
    let (pitch_sin, pitch_cos) = (-pitch).to_radians().sin_cos();

    Vec3::new(yaw_cos * pitch_cos, pitch_sin, yaw_sin * pitch_cos)
}

/// Returns the minimum number of bits needed to represent the integer `n`.
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
            let d = (Vec3::new(random(), random(), random()) * 2.0 - 1.0).normalize();

            let (yaw, pitch) = to_yaw_and_pitch(d);
            let d_new = from_yaw_and_pitch(yaw, pitch);

            assert_relative_eq!(d, d_new, epsilon = f32::EPSILON * 100.0);
        }
    }
}
