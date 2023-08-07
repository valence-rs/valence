use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTITY_POSITION_S2C)]
pub struct EntityPositionS2c {
    pub entity_id: VarInt,
    pub position: DVec3,
    pub yaw: ByteAngle,
    pub pitch: ByteAngle,
    pub on_ground: bool,
}
