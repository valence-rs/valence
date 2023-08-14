use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTITY_STATUS_S2C)]
pub struct EntityStatusS2c {
    pub entity_id: i32,
    pub entity_status: u8,
}
