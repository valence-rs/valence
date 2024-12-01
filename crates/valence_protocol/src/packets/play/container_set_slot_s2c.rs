use std::borrow::Cow;

use crate::{Decode, Encode, ItemStack, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct ContainerSetSlotS2c<'a> {
    pub window_id: VarInt,
    pub state_id: VarInt,
    pub slot_idx: i16,
    pub slot_data: Cow<'a, ItemStack<'a>>,
}
