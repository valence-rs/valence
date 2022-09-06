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
            nbt,
        }
    }

    /// Constructs an empty slot.
    pub const fn empty() -> Self {
        Self {
            present: false,
            item_id: None,
            item_count: None,
            nbt: None,
        }
    }
}

impl Encode for Slot {
    fn encode(&self, w: &mut impl Write) -> anyhow::Result<()> {
        if !self.present {
            return self.present.encode(w);
        }
        self.present.encode(w)?;
        self.item_id
            .expect("If item is present then slot should have an item id")
            .encode(w)?;
        self.item_count
            .expect("If item is present then slot should have an item count")
            .encode(w)?;
        match &self.nbt {
            Some(n) => n.encode(w),
            None => 0u8.encode(w),
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

        assert!(decoded_slot.present);
        assert_eq!(1, decoded_slot.item_id.unwrap().0);
        assert_eq!(1, decoded_slot.item_count.unwrap());
        assert_eq!(
            Value::Int(1),
            *decoded_slot.nbt.unwrap().get("Unbreakable").unwrap()
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

        assert_eq!(None, decoded_slot.nbt);

        // `Cursor::is_empty()` is unstable :(
        assert!(cursor.position() >= cursor.get_ref().len() as u64);
    }

    #[test]
    fn empty_slot() {
        let mut buf: Vec<u8> = Vec::new();

        Slot::empty().encode(&mut buf).unwrap();

        let mut cursor = Cursor::new(buf.as_slice());
        let decoded_slot = Slot::decode(&mut cursor).unwrap();

        assert!(!decoded_slot.present);
        assert_eq!(None, decoded_slot.item_id);

        // `Cursor::is_empty()` is unstable :(
        assert!(cursor.position() >= cursor.get_ref().len() as u64);
    }
}
