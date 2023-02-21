use std::borrow::Cow;

use crate::item::ItemStack;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x10]
pub struct InventoryS2c<'a> {
    pub window_id: u8,
    pub state_id: VarInt,
    pub slots: Cow<'a, [Option<ItemStack>]>,
    pub carried_item: Cow<'a, Option<ItemStack>>,
}
