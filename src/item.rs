//! Items and ItemStacks

use anyhow::Context;

use crate::block::{BlockKind, BlockKindType, CauldronBlockKind, WallBlockKind};
use crate::nbt::Compound;
use crate::protocol::{Decode, Encode, VarInt};

include!(concat!(env!("OUT_DIR"), "/item.rs"));

#[derive(Debug, Clone, PartialEq)]
pub struct ItemStack {
    pub item: ItemKind,
    pub item_count: u8,
    pub nbt: Option<Compound>,
}

impl Encode for ItemKind {
    fn encode(&self, w: &mut impl std::io::Write) -> anyhow::Result<()> {
        VarInt(self.to_raw() as i32).encode(w)
    }
}

impl Decode for ItemKind {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        let id = VarInt::decode(r)?.0;
        let errmsg = "invalid item ID";

        ItemKind::from_raw(id.try_into().context(errmsg)?).context(errmsg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_kind_to_block_kind() {
        let item = ItemKind::OakWood;

        let new_block = match BlockKind::from_item(item).unwrap() {
            BlockKindType::Normal(b) => b,
            _ => panic!(),
        };

        assert_eq!(new_block, BlockKind::OakWood)
    }

    #[test]
    fn block_state_to_item() {
        let block = BlockKind::SlimeBlock;

        let new_item = block.to_item().unwrap();

        assert_eq!(new_item, ItemKind::SlimeBlock)
    }
}
