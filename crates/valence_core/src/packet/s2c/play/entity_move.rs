use crate::packet::byte_angle::ByteAngle;
use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct MoveRelative {
    pub entity_id: VarInt,
    pub delta: [i16; 3],
    pub on_ground: bool,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct RotateAndMoveRelative {
    pub entity_id: VarInt,
    pub delta: [i16; 3],
    pub yaw: ByteAngle,
    pub pitch: ByteAngle,
    pub on_ground: bool,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct Rotate {
    pub entity_id: VarInt,
    pub yaw: ByteAngle,
    pub pitch: ByteAngle,
    pub on_ground: bool,
}
