use std::borrow::Cow;
use std::io::Write;

use crate::ident;
use crate::ident::Ident;
use crate::protocol::var_int::VarInt;
use crate::protocol::{Decode, Encode};

include!(concat!(env!("OUT_DIR"), "/sound.rs"));

impl Sound {
    pub fn to_id(self) -> SoundId<'static> {
        SoundId::Direct {
            id: self.to_ident().into(),
            range: None,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub enum SoundId<'a> {
    Direct {
        id: Ident<Cow<'a, str>>,
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
                id: Ident::decode(r)?,
                range: <Option<f32>>::decode(r)?,
            })
        } else {
            Ok(SoundId::Reference { id: VarInt(i - 1) })
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum SoundCategory {
    Master,
    Music,
    Record,
    Weather,
    Block,
    Hostile,
    Neutral,
    Player,
    Ambient,
    Voice,
}
