use crate::{Decode, Encode, ItemStack, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct SetCursorItemS2c<'a> {
    item: ItemStack<'a>,
}
