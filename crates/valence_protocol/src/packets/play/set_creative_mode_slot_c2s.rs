use crate::{Decode, Encode, ItemStack, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct SetCreativeModeSlotC2s<'a> {
    pub slot: i16,
    pub clicked_item: ItemStack<'a>,
}
