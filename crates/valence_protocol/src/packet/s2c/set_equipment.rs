use std::io::Write;

use crate::{Decode, DecodePacket, Encode, EncodePacket, ItemStack, VarInt};

#[derive(Clone, PartialEq, Debug, EncodePacket, DecodePacket)]
#[packet_id = 0x51]
pub struct SetEquipment {
    pub entity_id: VarInt,
    pub equipment: Vec<EquipmentEntry>,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct EquipmentEntry {
    pub slot: i8,
    pub item: Option<ItemStack>,
}

impl Encode for SetEquipment {
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

impl<'a> Decode<'a> for SetEquipment {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let entity_id = VarInt::decode(r)?;

        let mut equipment = vec![];

        loop {
            let slot = i8::decode(r)?;
            let item = Option::<ItemStack>::decode(r)?;
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
