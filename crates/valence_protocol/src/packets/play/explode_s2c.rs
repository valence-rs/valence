use valence_math::{DVec3, Vec3};

use crate::{sound::SoundId, Decode, Encode, Packet, Particle};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct ExplodeS2c {
    pub pos: DVec3,
    pub player_motion: Option<Vec3>,
    pub particle: Particle,
    pub sound: SoundId,
}
