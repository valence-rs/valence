use valence_math::{DVec3, Vec3};

use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct ExplosionS2c {
    pub pos: DVec3,
    pub strength: f32,
    pub affected_blocks: Vec<[i8; 3]>,
    pub player_motion: Vec3,
}
