use std::borrow::Cow;

use crate::item::ItemStack;
use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct InventoryS2c<'a> {
    pub window_id: u8,
    pub state_id: VarInt,
    pub slots: Cow<'a, [Option<ItemStack>]>,
    pub carried_item: Cow<'a, Option<ItemStack>>,
}
