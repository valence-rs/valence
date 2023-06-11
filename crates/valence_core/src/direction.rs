use bevy_ecs::prelude::*;

use crate::protocol::{Decode, Encode};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode, Component)]
pub enum Direction {
    /// -Y
    Down,
    /// +Y
    Up,
    /// -Z
    North,
    /// +Z
    South,
    /// -X
    West,
    /// +X
    East,
}
