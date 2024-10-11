use std::io::Write;

use serde::{Deserialize, Serialize};
pub use valence_generated::item::ItemKind;
use valence_nbt::Compound;

use crate::{Decode, Encode};

/// A stack of items in an inventory.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
pub struct ItemStack {
    pub item: ItemKind,
    pub count: i8,
    pub nbt: Option<Compound>,
}

impl ItemStack {
    pub const EMPTY: ItemStack = ItemStack {
        item: ItemKind::Air,
        count: 0,
        nbt: None,
    };

    #[must_use]
    pub const fn new(item: ItemKind, count: i8, nbt: Option<Compound>) -> Self {
        Self { item, count, nbt }
    }

    #[must_use]
    pub const fn with_count(mut self, count: i8) -> Self {
        self.count = count;
        self
    }

    #[must_use]
    pub const fn with_item(mut self, item: ItemKind) -> Self {
        self.item = item;
        self
    }

    #[must_use]
    pub fn with_nbt<C: Into<Option<Compound>>>(mut self, nbt: C) -> Self {
        self.nbt = nbt.into();
        self
    }

    pub const fn is_empty(&self) -> bool {
        matches!(self.item, ItemKind::Air) || self.count <= 0
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
                None => 0_u8.encode(w),
            }
        }
    }
}

impl Decode<'_> for ItemStack {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        let present = bool::decode(r)?;
        if !present {
            return Ok(ItemStack::EMPTY);
        };

        let item = ItemKind::decode(r)?;
        let count = i8::decode(r)?;

        let nbt = if let [0, rest @ ..] = *r {
            *r = rest;
            None
        } else {
            Some(Compound::decode(r)?)
        };

        let stack = ItemStack { item, count, nbt };

        // Normalize empty item stacks.
        if stack.is_empty() {
            Ok(ItemStack::EMPTY)
        } else {
            Ok(stack)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_item_stack_is_empty() {
        let air_stack = ItemStack::new(ItemKind::Air, 10, None);
        let less_then_one_stack = ItemStack::new(ItemKind::Stone, 0, None);

        assert!(air_stack.is_empty());
        assert!(less_then_one_stack.is_empty());

        assert!(ItemStack::EMPTY.is_empty());

        let not_empty_stack = ItemStack::new(ItemKind::Stone, 10, None);

        assert!(!not_empty_stack.is_empty());
    }
}
