use std::io::Write;

use byteorder::ReadBytesExt;

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

#[cfg(test)]
mod tests {
    use serde_nbt::Value;

    use super::*;

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

        Slot::Present {
            item_id: VarInt(1),
            item_count: 1,
            nbt: Some(nbt),
        }
        .encode(&mut buf)
        .unwrap();

        let mut slice = buf.as_slice();
        let (item_id, item_count, nbt) = match Slot::decode(&mut slice).unwrap() {
            Slot::Empty => {
                panic!("Slot should be present")
            }
            Slot::Present {
                item_id,
                item_count,
                nbt,
            } => (item_id, item_count, nbt),
        };
        assert_eq!(1, item_id.0);
        assert_eq!(1, item_count);
        assert_eq!(&Value::Int(1), nbt.unwrap().get("Unbreakable").unwrap());

        assert!(slice.is_empty());
    }

    #[test]
    fn slot_no_nbt() {
        let mut buf: Vec<u8> = Vec::new();

        Slot::Present {
            item_id: VarInt(1),
            item_count: 1,
            nbt: None,
        }
        .encode(&mut buf)
        .unwrap();

        let mut slice = buf.as_slice();
        let nbt = match Slot::decode(&mut slice).unwrap() {
            Slot::Empty => {
                panic!("Slot should be present")
            }
            Slot::Present { nbt, .. } => nbt,
        };

        assert_eq!(None, nbt);

        assert!(slice.is_empty());
    }

    #[test]
    fn empty_slot() {
        let mut buf: Vec<u8> = Vec::new();

        Slot::Empty.encode(&mut buf).unwrap();

        let mut slice = buf.as_slice();
        if let Slot::Present { .. } = Slot::decode(&mut slice).unwrap() {
            panic!("Slot should be empty")
        };

        assert!(slice.is_empty());
    }
}
