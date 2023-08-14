use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PICK_FROM_INVENTORY_C2S)]
pub struct PickFromInventoryC2s {
    pub slot_to_use: VarInt,
}
