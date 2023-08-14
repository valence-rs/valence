use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTITY_SET_HEAD_YAW_S2C)]
pub struct EntitySetHeadYawS2c {
    pub entity_id: VarInt,
    pub head_yaw: ByteAngle,
}
