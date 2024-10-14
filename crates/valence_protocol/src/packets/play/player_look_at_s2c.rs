use valence_math::DVec3;

use crate::{Decode, Encode, Packet, VarInt};

/// Instructs a client to face an entity.
#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode, Packet)]
pub struct PlayerLookAtS2c {
    pub feet_or_eyes: FeetOrEyes,
    pub target_position: DVec3,
    pub entity_to_face: Option<LookAtEntity>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum FeetOrEyes {
    Feet,
    Eyes,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub struct LookAtEntity {
    pub entity_id: VarInt,
    pub feet_or_eyes: FeetOrEyes,
}
