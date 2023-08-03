use super::*;

/// Spawns one or more experience orbs.
///
/// wiki : [Spawn Experience Orb](https://wiki.vg/Protocol#Spawn_Experience_Orb)
#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::EXPERIENCE_ORB_SPAWN_S2C)]
pub struct ExperienceOrbSpawnS2c {
    pub entity_id: VarInt,
    pub position: DVec3,
    /// The amount of experience this orb will reward once collected.
    pub count: i16,
}
