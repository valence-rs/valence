use crate::item::ItemKind;
use crate::nbt::Compound;

#[derive(Debug, Clone, PartialEq)]
pub struct ItemStack {
    pub item: ItemKind,
    pub item_count: u8,
    pub nbt: Option<Compound>,
}
