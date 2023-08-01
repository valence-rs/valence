use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SET_CAMERA_ENTITY_S2C)]
pub struct SetCameraEntityS2c {
    pub entity_id: VarInt,
}
