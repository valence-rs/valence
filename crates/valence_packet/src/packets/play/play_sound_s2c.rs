use valence_core::sound::{SoundCategory, SoundId};

use super::*;

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
