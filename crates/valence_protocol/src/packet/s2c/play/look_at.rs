use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, PartialEq, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x37]
pub struct LookAtS2c {
    pub feet_or_eyes: FeetOrEyes,
    pub target_position: [f64; 3],
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
