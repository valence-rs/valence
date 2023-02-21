use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x19]
pub struct EntityStatusS2c {
    pub entity_id: i32,
    pub entity_status: u8,
}
