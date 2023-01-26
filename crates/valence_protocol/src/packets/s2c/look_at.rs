use std::io::Write;

use crate::{Decode, DecodePacket, Encode, EncodePacket, VarInt};

#[derive(Copy, Clone, PartialEq, Debug, EncodePacket, DecodePacket)]
#[packet_id = 0x37]
pub struct LookAt {
    pub feet_eyes: FeetEyes,
    pub target_position: [f64; 3],
    pub entity_id: Option<VarInt>,
    pub entity_feet_eyes: Option<FeetEyes>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum FeetEyes {
    Feet,
    Eyes,
}

impl Encode for LookAt {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        self.feet_eyes.encode(&mut w)?;
        self.target_position.encode(&mut w)?;

        if let Some(entity_id) = self.entity_id {
            true.encode(&mut w)?;
            entity_id.encode(&mut w)?;

            if let Some(entity_feet_eyes) = self.entity_feet_eyes {
                entity_feet_eyes.encode(&mut w)?;
            } else {
                FeetEyes::Feet.encode(&mut w)?;
            }
        } else {
            false.encode(&mut w)?;
        }

        Ok(())
    }
}

impl<'a> Decode<'a> for LookAt {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let feet_eyes = FeetEyes::decode(r)?;
        let target_position = <[f64; 3]>::decode(r)?;

        let (entity_id, entity_feet_eyes) = if bool::decode(r)? {
            (Some(VarInt::decode(r)?), Some(FeetEyes::decode(r)?))
        } else {
            (None, None)
        };

        Ok(Self {
            feet_eyes,
            target_position,
            entity_id,
            entity_feet_eyes,
        })
    }
}
