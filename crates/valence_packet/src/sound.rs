use valence_core::sound::{SoundCategory, SoundId};

use super::*;

#[cfg(test)]
mod tests {
    use valence_core::ident;
    use valence_core::sound::Sound;

    use super::*;

    #[test]
    fn sound_to_soundid() {
        assert_eq!(
            Sound::BlockBellUse.to_id(),
            SoundId::Direct {
                id: ident!("block.bell.use").into(),
                range: None
            },
        );
    }
}

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAY_SOUND_FROM_ENTITY_S2C)]
pub struct PlaySoundFromEntityS2c {
    pub id: VarInt,
    pub category: SoundCategory,
    pub entity_id: VarInt,
    pub volume: f32,
    pub pitch: f32,
    pub seed: i64,
}

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAY_SOUND_S2C)]
pub struct PlaySoundS2c<'a> {
    pub id: SoundId<'a>,
    pub category: SoundCategory,
    pub position: IVec3,
    pub volume: f32,
    pub pitch: f32,
    pub seed: i64,
}

#[derive(Clone, PartialEq, Debug, Packet)]
#[packet(id = packet_id::STOP_SOUND_S2C)]
pub struct StopSoundS2c<'a> {
    pub source: Option<SoundCategory>,
    pub sound: Option<Ident<Cow<'a, str>>>,
}

impl Encode for StopSoundS2c<'_> {
    fn encode(&self, mut w: impl Write) -> anyhow::Result<()> {
        match (self.source, self.sound.as_ref()) {
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
                Some(<Ident<Cow<'a, str>>>::decode(r)?),
            ),
            2 => (None, Some(<Ident<Cow<'a, str>>>::decode(r)?)),
            1 => (Some(SoundCategory::decode(r)?), None),
            _ => (None, None),
        };

        Ok(Self { source, sound })
    }
}
