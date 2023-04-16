use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct SelectMerchantTradeC2s {
    pub selected_slot: VarInt,
}
