use crate::{Decode, Encode, Packet, VarInt};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct SlotChangedStateC2s {
    pub slot_id: VarInt,
    pub window_id: VarInt,
    pub state: bool,
}
