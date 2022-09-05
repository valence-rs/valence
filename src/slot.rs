use std::io::{Read, Seek, SeekFrom, Write};

use crate::nbt::Compound;
use crate::protocol::{Decode, Encode, VarInt};

/// Represents a slot in an inventory.
#[derive(Clone, Default, Debug)]
pub struct Slot {
    pub present: bool,
    pub item_id: Option<VarInt>,
    pub item_count: Option<u8>,
    pub nbt: Option<Compound>,
}

impl Slot {
    /// Constructs a new slot.
    pub const fn new(item_id: VarInt, item_count: u8, nbt: Option<Compound>) -> Self {
        Self {
            present: true,
            item_id: Some(item_id),
            item_count: Some(item_count),
            nbt
        }
    }

    /// Constructs an empty slot.
    pub const fn empty() -> Self {
        Self {
            present: false,
            item_id: None,
            item_count: None,
            nbt: None
        }
    }
}

impl Encode for Slot {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        if !self.present {
            return self.present.encode(w);
        }
        self.present.encode(w)?;
        self.item_id.encode(w)?;
        self.item_count.encode(w)?;
        self.nbt.encode(w)
    }
}

impl Decode for Slot {
    fn decode(r: &mut (impl Read + Seek)) -> anyhow::Result<Self> {
        let present = bool::decode(r)?;
        if !present {
            return Ok(Slot::empty())
        }
        Ok(Slot::new(
            VarInt::decode(r)?,
            u8::decode(r)?,
            if u8::decode(r)? == 0 {
                None
            } else {
                r.seek(SeekFrom::Current(-1))?;
                Some(Compound::decode(r)?)
            }
        ))
    }
}
