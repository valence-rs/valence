use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct ExperienceOrbSpawnS2c {
    pub entity_id: VarInt,
    pub position: [f64; 3],
    pub count: i16,
}
