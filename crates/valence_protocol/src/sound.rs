// sound.rs exposes constant values provided by the build script.
// All sounds are located in `Sound`. You can use the
// associated const fn functions of `Sound` to access details about a sound.
include!(concat!(env!("OUT_DIR"), "/sound.rs"));

use crate::packets::s2c::play::SoundId;
use crate::Ident;

impl Sound {
    pub fn to_id(self) -> SoundId<'static> {
        SoundId::Direct {
            id: Ident::new(self.to_str()).unwrap(),
            range: None,
        }
    }
}
