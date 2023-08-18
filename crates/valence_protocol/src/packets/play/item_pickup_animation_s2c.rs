use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct ItemPickupAnimationS2c {
    pub collected_entity_id: VarInt,
    pub collector_entity_id: VarInt,
    pub pickup_item_count: VarInt,
}
