use crate::byte_angle::ByteAngle;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x3e]
pub struct EntitySetHeadYawS2c {
    pub entity_id: VarInt,
    pub head_yaw: ByteAngle,
}
