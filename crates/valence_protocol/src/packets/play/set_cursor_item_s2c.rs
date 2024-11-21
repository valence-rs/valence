use crate::{Decode, Encode, ItemStack, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct SetCursorItemS2c {
    item: ItemStack,
}
