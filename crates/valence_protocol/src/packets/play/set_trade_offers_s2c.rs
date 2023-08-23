use crate::{Decode, Encode, ItemStack, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct SetTradeOffersS2c {
    pub window_id: VarInt,
    pub trades: Vec<TradeOffer>,
    pub villager_level: VarInt,
    pub experience: VarInt,
    pub is_regular_villager: bool,
    pub can_restock: bool,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct TradeOffer {
    pub input_one: Option<ItemStack>,
    pub output_item: Option<ItemStack>,
    pub input_two: Option<ItemStack>,
    pub trade_disabled: bool,
    pub number_of_trade_uses: i32,
    pub max_trade_uses: i32,
    pub xp: i32,
    pub special_price: i32,
    pub price_multiplier: f32,
    pub demand: i32,
}
