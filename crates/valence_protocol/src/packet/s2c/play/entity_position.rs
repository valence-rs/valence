use crate::byte_angle::ByteAngle;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x64]
pub struct EntityPositionS2c {
    pub entity_id: VarInt,
    pub position: [f64; 3],
    pub yaw: ByteAngle,
    pub pitch: ByteAngle,
    pub on_ground: bool,
}
