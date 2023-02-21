use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x63]
pub struct ItemPickupAnimationS2c {
    pub collected_entity_id: VarInt,
    pub collector_entity_id: VarInt,
    pub pickup_item_count: VarInt,
}
