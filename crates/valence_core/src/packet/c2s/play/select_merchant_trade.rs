use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct SelectMerchantTradeC2s {
    pub selected_slot: VarInt,
}
