use crate::{Decode, Encode, Packet, VarInt};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct PickFromInventoryC2s {
    pub slot_to_use: VarInt,
}
