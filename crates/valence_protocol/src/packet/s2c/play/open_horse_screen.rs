use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x1d]
pub struct OpenHorseScreenS2c {
    pub window_id: u8,
    pub slot_count: VarInt,
    pub entity_id: i32,
}
