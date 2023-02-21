use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x26]
pub struct SelectMerchantTradeC2s {
    pub selected_slot: VarInt,
}
