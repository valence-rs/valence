use crate::{Decode, Encode, Packet, VarInt};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct SelectMerchantTradeC2s {
    pub selected_slot: VarInt,
}
