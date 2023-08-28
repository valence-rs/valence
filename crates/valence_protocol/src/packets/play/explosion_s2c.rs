use std::borrow::Cow;

use valence_math::DVec3;

use crate::{BlockPos, Decode, Encode, Packet, Velocity};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct ExplosionS2c<'a> {
    pub pos: DVec3,
    pub radius: f32,
    pub affected_blocks: Cow<'a, [BlockPos]>,
    pub player_velocity: Velocity,
}
