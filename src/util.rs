//! Miscellaneous utilities.

use std::iter::FusedIterator;

use num::cast::AsPrimitive;
use num::Float;
use vek::{Aabb, Vec3};

use crate::chunk_pos::ChunkPos;

/// Returns true if the given string meets the criteria for a valid Minecraft
/// username.
///
/// Usernames are valid if they match the regex `^[a-zA-Z0-9_]{3,16}$`.
///
/// # Examples
/// 
/// ```
/// use valence::util::valid_username;
///
/// assert!(valid_username("00a"));
/// assert!(valid_username("jeb_"));
///
/// assert!(!valid_username("notavalidusername"));
/// assert!(!valid_username("NotValid!"))
/// ```
pub fn valid_username(s: &str) -> bool {
    (3..=16).contains(&s.len())
        && s.chars()
            .all(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_'))
}

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
            bottom.y,
            bottom.z + size.z / 2.0.as_(),
        ),
    };

    debug_assert!(aabb.is_valid());

    aabb
}

/// Takes a normalized direction vector and returns a `(yaw, pitch)` tuple in
/// degrees.
///
// /// This function is the inverse of [`from_yaw_and_pitch`].
pub fn to_yaw_and_pitch(d: Vec3<f64>) -> (f32, f32) {
    debug_assert!(d.is_normalized(), "the given vector should be normalized");

    let yaw = f32::atan2(d.z as f32, d.x as f32).to_degrees() - 90.0;
    let pitch = -(d.y as f32).asin().to_degrees();
    (yaw, pitch)
}

// /// Takes yaw and pitch angles (in degrees) and returns a normalized
// direction /// vector.
// ///
// /// This function is the inverse of [`to_yaw_and_pitch`].
// pub fn from_yaw_and_pitch(yaw: f32, pitch: f32) -> Vec3<f64> {
//     todo!()
// }
