use crate::ident;
use crate::ident::Ident;
use crate::packet::s2c::play::play_sound::SoundId;

include!(concat!(env!("OUT_DIR"), "/sound.rs"));

impl Sound {
    pub fn to_id(self) -> SoundId<'static> {
        SoundId::Direct {
            id: self.to_ident().into(),
            range: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_to_soundid() {
        assert_eq!(
            Sound::BlockBellUse.to_id(),
            SoundId::Direct {
                id: Ident::new("block.bell.use").unwrap(),
                range: None
            },
        );
    }
}
