use super::*;
use crate::sound::{SoundCategory, SoundId};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct PlaySoundS2c<'a> {
    pub id: SoundId<'a>,
    pub category: SoundCategory,
    pub position: IVec3,
    pub volume: f32,
    pub pitch: f32,
    pub seed: i64,
}
