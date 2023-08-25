use std::io::Write;

use anyhow::ensure;
pub use valence_generated::item::ItemKind;
use valence_nbt::Compound;

use crate::{Decode, Encode};

/// A stack of items in an inventory.
#[derive(Clone, PartialEq, Debug)]
pub struct ItemStack {
    pub item: ItemKind,
    pub count: i8,
    pub nbt: Option<Compound>,
}

impl ItemStack {
    pub const STACK_MIN: i8 = 1;
    pub const STACK_MAX: i8 = 127;

    #[must_use]
    pub fn new(item: ItemKind, count: i8, nbt: Option<Compound>) -> Self {
        Self {
            item,
            count: count.clamp(Self::STACK_MIN, Self::STACK_MAX),
            nbt,
        }
    }

    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_count(mut self, count: i8) -> Self {
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
    pub fn count(&self) -> i8 {
        self.count
    }

    /// Sets the number of items in this stack. Values are clamped to 1-127,
    /// which are the positive values accepted by clients.
    pub fn set_count(&mut self, count: i8) {
        self.count = count.clamp(Self::STACK_MIN, Self::STACK_MAX);
    }

    pub fn is_empty(&self) -> bool {
        self.item == ItemKind::Air || self.count <= 0
    }
}

impl Default for ItemStack {
    fn default() -> Self {
        Self::new(ItemKind::Air, 1, None)
    }
}

impl Encode for ItemStack {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        if self.is_empty() {
            false.encode(w)
        } else {
            true.encode(&mut w)?;
            self.item.encode(&mut w)?;
            self.count.encode(&mut w)?;
            match &self.nbt {
                Some(n) => n.encode(w),
                None => 0u8.encode(w),
            }
        }
    }
}

impl Decode<'_> for ItemStack {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        let present = bool::decode(r)?;
        if !present {
            return Ok(ItemStack::empty());
        };

        let item = ItemKind::decode(r)?;
        let count = i8::decode(r)?;

        ensure!(
            (ItemStack::STACK_MIN..=ItemStack::STACK_MAX).contains(&count),
            "invalid item stack count (got {count}, expected {}..={})",
            ItemStack::STACK_MIN,
            ItemStack::STACK_MAX,
        );

        let nbt = if let [0, rest @ ..] = *r {
            *r = rest;
            None
        } else {
            Some(Compound::decode(r)?)
        };

        Ok(ItemStack { item, count, nbt })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_item_stack_is_empty() {
        let default_stack = ItemStack::default();
        let air_stack = ItemStack::new(ItemKind::Air, 10, None);
        let less_then_one_stack = ItemStack::new(ItemKind::Stone, 0, None);

        assert!(default_stack.is_empty());
        assert!(air_stack.is_empty());
        assert!(less_then_one_stack.is_empty());
    }

    #[test]
    fn item_stack_clamps_count() {
        let mut stack = ItemStack::new(ItemKind::Stone, -30, None);
        assert_eq!(stack.count, ItemStack::STACK_MIN);

        stack.set_count(100);
        assert_eq!(stack.count, ItemStack::STACK_MAX);
    }
}
