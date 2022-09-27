use crate::nbt::Compound;
use crate::protocol::VarInt;

#[derive(Debug, Clone, PartialEq)]
pub struct ItemStack {
    pub item_id: VarInt,
    pub item_count: u8,
    pub nbt: Option<Compound>,
}
