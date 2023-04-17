use std::io::Write;

use anyhow::{ensure, Context};
use valence_nbt::Compound;

use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

include!(concat!(env!("OUT_DIR"), "/item.rs"));

#[derive(Clone, PartialEq, Debug)]
pub struct ItemStack {
    pub item: ItemKind,
    count: u8,
    pub nbt: Option<Compound>,
}

pub const STACK_MIN: u8 = 1;
pub const STACK_MAX: u8 = 127;

impl ItemStack {
    #[must_use]
    pub fn new(item: ItemKind, count: u8, nbt: Option<Compound>) -> Self {
        Self {
            item,
            count: count.clamp(STACK_MIN, STACK_MAX),
            nbt,
        }
    }

    #[must_use]
    pub fn with_count(mut self, count: u8) -> Self {
        self.set_count(count);
        self
    }

    #[must_use]
    pub fn with_item(mut self, item: ItemKind) -> Self {
        self.item = item;
        self
    }

    #[must_use]
    pub fn with_nbt(mut self, nbt: impl Into<Option<Compound>>) -> Self {
        self.nbt = nbt.into();
        self
    }

    /// Gets the number of items in this stack.
    pub fn count(&self) -> u8 {
        self.count
    }

    /// Sets the number of items in this stack. Values are clamped to 1-127,
    /// which are the positive values accepted by clients.
    pub fn set_count(&mut self, count: u8) {
        self.count = count.clamp(STACK_MIN, STACK_MAX);
    }
}

impl Default for ItemStack {
    fn default() -> Self {
        Self::new(ItemKind::Air, 1, None)
    }
}

impl Encode for Option<ItemStack> {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        self.as_ref().encode(w)
    }
}

impl<'a> Encode for Option<&'a ItemStack> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        match *self {
            None => false.encode(w),
            Some(s) => {
                true.encode(&mut w)?;
                s.item.encode(&mut w)?;
                s.count.encode(&mut w)?;
                match &s.nbt {
                    Some(n) => n.encode(w),
                    None => 0u8.encode(w),
                }
            }
        }
    }
}

impl Decode<'_> for Option<ItemStack> {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        let present = bool::decode(r)?;
        if !present {
            return Ok(None);
        }

        let item = ItemKind::decode(r)?;
        let count = u8::decode(r)?;

        ensure!(
            (STACK_MIN..=STACK_MAX).contains(&count),
            "invalid item stack count (got {count}, expected {STACK_MIN}..={STACK_MAX})"
        );

        let nbt = if let [0, rest @ ..] = *r {
            *r = rest;
            None
        } else {
            Some(Compound::decode(r)?)
        };

        Ok(Some(ItemStack { item, count, nbt }))
    }
}

impl Encode for ItemKind {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        VarInt(self.to_raw() as i32).encode(w)
    }
}

impl Decode<'_> for ItemKind {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        let id = VarInt::decode(r)?.0;
        let errmsg = "invalid item ID";

        ItemKind::from_raw(id.try_into().context(errmsg)?).context(errmsg)
    }
}

/*
#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::BlockKind;

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
        assert_eq!(stack.count, STACK_MAX);
        stack.set_count(201);
        assert_eq!(stack.count, STACK_MAX);
    }
}
*/
