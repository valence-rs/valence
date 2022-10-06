//! Items and ItemStacks

use anyhow::Context;

use crate::block::BlockKind;
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
        assert_eq!(
            ItemKind::Cauldron.to_block_kind(),
            Some(BlockKind::Cauldron)
        );
    }

    #[test]
    fn block_state_to_item() {
        assert_eq!(BlockKind::Torch.to_item_kind(), ItemKind::Torch);
        assert_eq!(BlockKind::WallTorch.to_item_kind(), ItemKind::Torch);

        assert_eq!(BlockKind::Cauldron.to_item_kind(), ItemKind::Cauldron);
        assert_eq!(BlockKind::LavaCauldron.to_item_kind(), ItemKind::Cauldron);

        assert_eq!(BlockKind::NetherPortal.to_item_kind(), ItemKind::Air);
    }
}
