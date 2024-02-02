use valence_math::DVec3;

use crate::{Decode, Encode, Packet, VarInt};

/// Spawns one or more experience orbs.
///
/// wiki : [Spawn Experience Orb](https://wiki.vg/Protocol#Spawn_Experience_Orb)
#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct ExperienceOrbSpawnS2c {
    pub entity_id: VarInt,
    pub position: DVec3,
    /// The amount of experience this orb will reward once collected.
    pub count: i16,
}
