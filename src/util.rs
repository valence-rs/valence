use std::iter::FusedIterator;

use num::cast::AsPrimitive;
use num::Float;
use vek::{Aabb, Extent3, Vec3};

use crate::ChunkPos;

/// Returns true if the given string meets the criteria for a valid Minecraft
/// username.
pub fn valid_username(s: &str) -> bool {
    (3..=16).contains(&s.len())
        && s.chars()
            .all(|c| matches!(c, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_'))
}

const EXTRA_RADIUS: i32 = 3;

pub fn chunks_in_view_distance(
    center: ChunkPos,
    distance: u8,
) -> impl FusedIterator<Item = ChunkPos> {
    let dist = distance as i32 + EXTRA_RADIUS;
    (center.z - dist..=center.z + dist)
        .flat_map(move |z| (center.x - dist..=center.x + dist).map(move |x| ChunkPos { x, z }))
        .filter(move |&p| is_chunk_in_view_distance(center, p, distance))
}

pub fn is_chunk_in_view_distance(center: ChunkPos, other: ChunkPos, distance: u8) -> bool {
    (center.x as f64 - other.x as f64).powi(2) + (center.z as f64 - other.z as f64).powi(2)
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
