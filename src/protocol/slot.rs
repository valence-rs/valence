use std::io::{Read, Seek, SeekFrom, Write};

use crate::nbt::Compound;
use crate::protocol::{Decode, Encode, VarInt};

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

impl Slot {
    /// Constructs a new slot.
    pub const fn new(item_id: VarInt, item_count: u8, nbt: Option<Compound>) -> Self {
        Self::Present {
            item_id,
            item_count,
            nbt,
        }
    }

    /// Constructs an empty slot.
    pub const fn empty() -> Self {
        Self::Empty
    }

    /// Returns `true` if there is an item present.
    pub const fn is_present(&self) -> bool {
        match self {
            Slot::Empty => false,
            Slot::Present { .. } => true,
        }
    }

    /// Gets the item id.
    ///
    /// If the slot is empty, then `None` is returned.
    pub fn item_id(&self) -> Option<&VarInt> {
        match self {
            Slot::Empty => None,
            Slot::Present { item_id, .. } => Some(item_id),
        }
    }

    /// Gets the item count.
    ///
    /// If the slot is empty, then `None` is returned.
    pub fn item_count(&self) -> Option<&u8> {
        match self {
            Slot::Empty => None,
            Slot::Present { item_count, .. } => Some(item_count),
        }
    }

    /// Gets the item's nbt data.
    ///
    /// If the slot is empty or there is no nbt data, then `None` is returned.
    pub fn nbt(&self) -> Option<&Compound> {
        match self {
            Slot::Empty => None,
            Slot::Present { nbt, .. } => nbt.as_ref(),
        }
    }
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
    fn decode(r: &mut (impl Read + Seek)) -> anyhow::Result<Self> {
        let present = bool::decode(r)?;
        if !present {
            return Ok(Slot::empty());
        }
        Ok(Slot::new(
            VarInt::decode(r)?,
            u8::decode(r)?,
            match u8::decode(r)? {
                0 => None,
                _ => {
                    r.seek(SeekFrom::Current(-1))?;
                    Some(Compound::decode(r)?)
                }
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_nbt::Value;
    use std::io::Cursor;

    #[test]
    fn slot_with_nbt() {
        let mut buf: Vec<u8> = Vec::new();

        // Example nbt blob
        // https://wiki.vg/Slot_Data
        let mut nbt = Compound::new();
        {
            let mut enchant = Compound::new();
            enchant.insert("id".to_string(), Value::Short(1));
            enchant.insert("lvl".to_string(), Value::Short(1));

            let enchant_list = vec![enchant];
            nbt.insert(
                "StoredEnchantments".to_string(),
                Value::List(enchant_list.into()),
            );
            nbt.insert("Unbreakable".to_string(), Value::Int(1));
        }

        Slot::new(VarInt(1), 1, Some(nbt)).encode(&mut buf).unwrap();

        let mut cursor = Cursor::new(buf.as_slice());
        let decoded_slot = Slot::decode(&mut cursor).unwrap();

        assert!(decoded_slot.is_present());
        assert_eq!(1, decoded_slot.item_id().unwrap().0);
        assert_eq!(1, *decoded_slot.item_count().unwrap());
        assert_eq!(
            &Value::Int(1),
            decoded_slot.nbt().unwrap().get("Unbreakable").unwrap()
        );

        // `Cursor::is_empty()` is unstable :(
        assert!(cursor.position() >= cursor.get_ref().len() as u64);
    }

    #[test]
    fn slot_no_nbt() {
        let mut buf: Vec<u8> = Vec::new();

        Slot::new(VarInt(1), 1, None).encode(&mut buf).unwrap();

        let mut cursor = Cursor::new(buf.as_slice());
        let decoded_slot = Slot::decode(&mut cursor).unwrap();

        assert_eq!(None, decoded_slot.nbt());

        // `Cursor::is_empty()` is unstable :(
        assert!(cursor.position() >= cursor.get_ref().len() as u64);
    }

    #[test]
    fn empty_slot() {
        let mut buf: Vec<u8> = Vec::new();

        Slot::empty().encode(&mut buf).unwrap();

        let mut cursor = Cursor::new(buf.as_slice());
        let decoded_slot = Slot::decode(&mut cursor).unwrap();

        assert!(!decoded_slot.is_present());
        assert_eq!(None, decoded_slot.item_id());

        // `Cursor::is_empty()` is unstable :(
        assert!(cursor.position() >= cursor.get_ref().len() as u64);
    }
}
