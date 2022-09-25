use std::io::Write;

use byteorder::ReadBytesExt;

use crate::nbt::Compound;
use crate::protocol::{Decode, Encode, VarInt};

pub type SlotId = i16;

/// Represents a slot in an inventory.
#[derive(Clone, Default, Debug)]
pub enum Slot {
    #[default]
    Empty,
    Present {
        item_id: VarInt,
        item_count: u8,
        nbt: Option<Compound>,
    },
}

impl Encode for Slot {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        match self {
            Slot::Empty => false.encode(w),
            Slot::Present {
                item_id,
                item_count,
                nbt,
            } => {
                true.encode(w)?;
                item_id.encode(w)?;
                item_count.encode(w)?;
                match &nbt {
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
        Ok(Slot::Present {
            item_id: VarInt::decode(r)?,
            item_count: u8::decode(r)?,
            nbt: if r.first() == Some(&0) {
                r.read_u8()?;
                None
            } else {
                Some(Compound::decode(r)?)
            },
        })
    }
}
