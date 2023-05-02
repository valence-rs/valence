use crate::ident;
use crate::ident::Ident;
use crate::packet::s2c::play::play_sound::SoundId;
use crate::packet::{Decode, Encode};

include!(concat!(env!("OUT_DIR"), "/sound.rs"));

impl Sound {
    pub fn to_id(self) -> SoundId<'static> {
        SoundId::Direct {
            id: self.to_ident().into(),
            range: None,
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

#[cfg(test)]
mod tests {
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
