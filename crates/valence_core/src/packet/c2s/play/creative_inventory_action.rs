use crate::item::ItemStack;
use crate::packet::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct CreativeInventoryActionC2s {
    pub slot: i16,
    pub clicked_item: Option<ItemStack>,
}
