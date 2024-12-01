use std::io::Write;

use crate::{Decode, Encode, ItemStack, Packet, VarInt};

#[derive(Clone, PartialEq, Debug, Packet)]
pub struct SetEquipmentS2c<'a> {
    pub entity_id: VarInt,
    pub equipment: Vec<EquipmentEntry<'a>>,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct EquipmentEntry<'a> {
    pub slot: i8,
    pub item: ItemStack<'a>,
}

impl<'a> Encode for SetEquipmentS2c<'a> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        self.entity_id.encode(&mut w)?;

        for i in 0..self.equipment.len() {
            let slot = self.equipment[i].slot;
            if i != self.equipment.len() - 1 {
                (slot | -128).encode(&mut w)?;
            } else {
                slot.encode(&mut w)?;
            }
            self.equipment[i].item.encode(&mut w)?;
        }

        Ok(())
    }
}

impl<'a> Decode<'a> for SetEquipmentS2c<'a> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let entity_id = VarInt::decode(r)?;

        let mut equipment = vec![];

        loop {
            let slot = i8::decode(r)?;
            let item = ItemStack::decode(r)?;
            equipment.push(EquipmentEntry {
                slot: slot & 127,
                item,
            });
            if slot & -128 == 0 {
                break;
            }
        }

        Ok(Self {
            entity_id,
            equipment,
        })
    }
}
