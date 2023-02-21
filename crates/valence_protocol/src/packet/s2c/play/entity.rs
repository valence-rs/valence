use crate::byte_angle::ByteAngle;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct MoveRelativeS2c {
    pub entity_id: VarInt,
    pub delta: [i16; 3],
    pub on_ground: bool,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct RotateAndMoveRelativeS2c {
    pub entity_id: VarInt,
    pub delta: [i16; 3],
    pub yaw: ByteAngle,
    pub pitch: ByteAngle,
    pub on_ground: bool,
}

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct RotateS2c {
    pub entity_id: VarInt,
    pub yaw: ByteAngle,
    pub pitch: ByteAngle,
    pub on_ground: bool,
}
