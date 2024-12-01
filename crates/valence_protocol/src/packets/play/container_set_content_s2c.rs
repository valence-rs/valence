use std::borrow::Cow;

use crate::{Decode, Encode, ItemStack, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct ContainerSetContentS2c<'a> {
    pub window_id: VarInt,
    pub state_id: VarInt,
    pub slots: Cow<'a, [ItemStack<'a>]>,
    pub carried_item: Cow<'a, ItemStack<'a>>,
}
