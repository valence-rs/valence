use super::*;

/// Instructs a client to face an entity.
#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::LOOK_AT_S2C)]
pub struct LookAtS2c {
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
