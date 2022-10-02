use std::io::Write;

use byteorder::ReadBytesExt;

use crate::item::ItemKind;
use crate::itemstack::ItemStack;
use crate::nbt::Compound;
use crate::protocol::{Decode, Encode};

pub type SlotId = i16;

/// Represents a slot in an inventory.
#[derive(Clone, Default, Debug)]
pub enum Slot {
    #[default]
    Empty,
    Present(ItemStack),
}

impl Encode for Slot {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        match self {
            Slot::Empty => false.encode(w),
            Slot::Present(s) => {
                true.encode(w)?;
                s.item.encode(w)?;
                s.item_count.encode(w)?;
                match &s.nbt {
                    Some(n) => n.encode(w),
                    None => 0u8.encode(w),
                }
            }
        }
    }
}

impl Decode for Slot {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        let present = bool::decode(r)?;
        if !present {
            return Ok(Slot::Empty);
        }
        Ok(Slot::Present(ItemStack {
            item: ItemKind::decode(r)?,
            item_count: u8::decode(r)?,
            nbt: if r.first() == Some(&0) {
                r.read_u8()?;
                None
            } else {
                Some(Compound::decode(r)?)
            },
        }))
    }
}

impl From<Option<ItemStack>> for Slot {
    fn from(s: Option<ItemStack>) -> Self {
        if let Some(s) = s {
            Slot::Present(s)
        } else {
            Slot::Empty
        }
    }
}

impl From<Slot> for Option<ItemStack> {
    fn from(s: Slot) -> Self {
        if let Slot::Present(s) = s {
            Some(s)
        } else {
            None
        }
    }
}
