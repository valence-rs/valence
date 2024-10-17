use crate::{Decode, Encode, Packet, VarInt};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct SelectTradeC2s {
    pub selected_slot: VarInt,
}
