//! Items and ItemStacks

use anyhow::Context;

use crate::block::BlockKind;
use crate::nbt::Compound;
use crate::protocol::{Decode, Encode, VarInt};

include!(concat!(env!("OUT_DIR"), "/item.rs"));

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

/// Represents a stack of items, possibly with NBT data. The number of items the
/// stack contains is clamped to 1-127, which is provided in the constants
/// `STACK_MIN` and `STACK_MAX` respectively. **A stack cannot have zero
/// items.** If you are consuming the last item in a stack, you need to remove
/// the stack from the slot.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemStack {
    pub item: ItemKind,
    item_count: u8,
    pub nbt: Option<Compound>,
}

impl ItemStack {
    const STACK_MIN: u8 = 1;
    const STACK_MAX: u8 = 127;

    pub fn new(item: ItemKind, count: u8, nbt: Option<Compound>) -> Self {
        Self {
            item,
            item_count: count.clamp(Self::STACK_MIN, Self::STACK_MAX),
            nbt,
        }
    }

    /// Gets the number of items in this stack.
    pub fn count(&self) -> u8 {
        self.item_count
    }

    /// Sets the number of items in this stack. Values are clamped to 1-127,
    /// which are the positive values accepted by clients.
    pub fn set_count(&mut self, count: u8) {
        self.item_count = count.clamp(Self::STACK_MIN, Self::STACK_MAX)
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

    #[test]
    fn item_stack_clamps_count() {
        let mut stack = ItemStack::new(ItemKind::Stone, 200, None);
        assert_eq!(stack.item_count, ItemStack::STACK_MAX);
        stack.set_count(201);
        assert_eq!(stack.item_count, ItemStack::STACK_MAX);
    }
}
