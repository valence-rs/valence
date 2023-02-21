use std::io::Write;

use crate::ident::Ident;
use crate::types::SoundCategory;
use crate::{Decode, Encode};

#[derive(Clone, PartialEq, Debug)]
pub struct StopSoundS2c<'a> {
    pub source: Option<SoundCategory>,
    pub sound: Option<Ident<&'a str>>,
}

impl Encode for StopSoundS2c<'_> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        match (self.source, self.sound) {
            (Some(source), Some(sound)) => {
                3i8.encode(&mut w)?;
                source.encode(&mut w)?;
                sound.encode(&mut w)?;
            }
            (None, Some(sound)) => {
                2i8.encode(&mut w)?;
                sound.encode(&mut w)?;
            }
            (Some(source), None) => {
                1i8.encode(&mut w)?;
                source.encode(&mut w)?;
            }
            _ => 0i8.encode(&mut w)?,
        }

        Ok(())
    }
}

impl<'a> Decode<'a> for StopSoundS2c<'a> {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let (source, sound) = match i8::decode(r)? {
            3 => (
                Some(SoundCategory::decode(r)?),
                Some(<Ident<&'a str>>::decode(r)?),
            ),
            2 => (None, Some(<Ident<&'a str>>::decode(r)?)),
            1 => (Some(SoundCategory::decode(r)?), None),
            _ => (None, None),
        };

        Ok(Self { source, sound })
    }
}
