use std::borrow::Cow;

use crate::{Decode, Encode, ItemStack, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct ScreenHandlerSlotUpdateS2c<'a> {
    pub window_id: i8,
    pub state_id: VarInt,
    pub slot_idx: i16,
    pub slot_data: Cow<'a, Option<ItemStack>>,
}
