use std::borrow::Cow;

use valence_math::{DVec3, Vec3};

use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct ExplodeS2c<'a> {
    pub pos: DVec3,
    pub strength: f32,
    pub affected_blocks: Cow<'a, [[i8; 3]]>,
    pub player_motion: Vec3,
    //TODO: particle data and sound
}
