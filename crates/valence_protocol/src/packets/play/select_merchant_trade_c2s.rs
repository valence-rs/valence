use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SELECT_MERCHANT_TRADE_C2S)]
pub struct SelectMerchantTradeC2s {
    pub selected_slot: VarInt,
}
