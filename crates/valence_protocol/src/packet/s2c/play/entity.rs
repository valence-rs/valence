use crate::byte_angle::ByteAngle;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x27]
pub struct MoveRelativeS2c {
    pub entity_id: VarInt,
    pub delta: [i16; 3],
    pub on_ground: bool,
}

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x28]
pub struct RotateAndMoveRelativeS2c {
    pub entity_id: VarInt,
    pub delta: [i16; 3],
    pub yaw: ByteAngle,
    pub pitch: ByteAngle,
    pub on_ground: bool,
}

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x29]
pub struct RotateS2c {
    pub entity_id: VarInt,
    pub yaw: ByteAngle,
    pub pitch: ByteAngle,
    pub on_ground: bool,
}
