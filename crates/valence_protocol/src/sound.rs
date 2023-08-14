use std::borrow::Cow;
use std::io::Write;

pub use valence_generated::sound::Sound;
use valence_ident::Ident;

use crate::var_int::VarInt;
use crate::{Decode, Encode};

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
