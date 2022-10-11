use std::io::Write;

use byteorder::ReadBytesExt;

use crate::item::{ItemKind, ItemStack};
use crate::nbt::Compound;
use crate::protocol::{Decode, Encode};

pub type SlotId = i16;

/// Represents a slot in an inventory.
pub type Slot = Option<ItemStack>;

impl Encode for Slot {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        match self {
            None => false.encode(w),
            Some(s) => {
                true.encode(w)?;
                s.item.encode(w)?;
                s.count().encode(w)?;
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
            return Ok(None);
        }
        Ok(Some(ItemStack::new(
            ItemKind::decode(r)?,
            u8::decode(r)?,
            if r.first() == Some(&0) {
                r.read_u8()?;
                None
            } else {
                Some(Compound::decode(r)?)
            },
        )))
    }
}
