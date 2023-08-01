use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::OPEN_HORSE_SCREEN_S2C)]
pub struct OpenHorseScreenS2c {
    pub window_id: u8,
    pub slot_count: VarInt,
    pub entity_id: i32,
}
