use std::io::Write;

use crate::ident::Ident;
use crate::types::SoundCategory;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x5e]
pub struct PlaySoundS2c<'a> {
    pub id: SoundId<'a>,
    pub category: SoundCategory,
    pub position: [i32; 3],
    pub volume: f32,
    pub pitch: f32,
    pub seed: i64,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum SoundId<'a> {
    Direct {
        id: Ident<&'a str>,
        range: Option<f32>,
    },
    Reference {
        id: VarInt,
    },
}

impl Encode for SoundId<'_> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        match self {
            SoundId::Direct { id, range } => {
                VarInt(0).encode(&mut w)?;
                id.encode(&mut w)?;
                range.encode(&mut w)?;
            }
            SoundId::Reference { id } => VarInt(id.0 + 1).encode(&mut w)?,
        }

        Ok(())
    }
}

impl<'a> Decode<'a> for SoundId<'a> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let i = VarInt::decode(r)?.0;

        if i == 0 {
            Ok(SoundId::Direct {
                id: <Ident<&'a str>>::decode(r)?,
                range: <Option<f32>>::decode(r)?,
            })
        } else {
            Ok(SoundId::Reference { id: VarInt(i - 1) })
        }
    }
}
