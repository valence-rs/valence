use crate::{Decode, Encode, ItemStack, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct CreativeInventoryActionC2s {
    pub slot: i16,
    pub clicked_item: Option<ItemStack>,
}
