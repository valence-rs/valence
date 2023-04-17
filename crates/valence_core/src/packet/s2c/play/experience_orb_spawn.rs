use glam::DVec3;

use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct ExperienceOrbSpawnS2c {
    pub entity_id: VarInt,
    pub position: DVec3,
    pub count: i16,
}
