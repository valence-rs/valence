use crate::item::Item;
use crate::nbt::Compound;

#[derive(Debug, Clone, PartialEq)]
pub struct ItemStack {
    pub item: Item,
    pub item_count: u8,
    pub nbt: Option<Compound>,
}
