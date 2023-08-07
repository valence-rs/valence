use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTITY_ATTACH_S2C)]
pub struct EntityAttachS2c {
    pub attached_entity_id: i32,
    pub holding_entity_id: i32,
}
