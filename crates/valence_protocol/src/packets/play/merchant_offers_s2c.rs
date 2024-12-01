use crate::{Decode, Encode, ItemStack, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct MerchantOffersS2c<'a> {
    pub window_id: VarInt,
    pub trades: Vec<TradeOffer<'a>>,
    pub villager_level: VarInt,
    pub experience: VarInt,
    pub is_regular_villager: bool,
    pub can_restock: bool,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct TradeOffer<'a> {
    pub input_one:   ItemStack<'a>,
    pub output_item: ItemStack<'a>,
    pub input_two:   ItemStack<'a>,
    pub trade_disabled: bool,
    pub number_of_trade_uses: i32,
    pub max_trade_uses: i32,
    pub xp: i32,
    pub special_price: i32,
    pub price_multiplier: f32,
    pub demand: i32,
}
