use std::borrow::Cow;

use crate::{Decode, Encode, ItemStack, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct SetPlayerInventoryS2c<'a> {
    pub slot: VarInt,
    pub slot_data: Cow<'a, ItemStack<'a>>,
}
