use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct ItemPickupAnimationS2c {
    pub collected_entity_id: VarInt,
    pub collector_entity_id: VarInt,
    pub pickup_item_count: VarInt,
}
