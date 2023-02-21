use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x4f]
pub struct EntityAttachS2c {
    pub attached_entity_id: i32,
    pub holding_entity_id: i32,
}
