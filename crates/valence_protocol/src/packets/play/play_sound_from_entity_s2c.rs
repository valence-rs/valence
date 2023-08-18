use super::*;
use crate::sound::SoundCategory;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct PlaySoundFromEntityS2c {
    pub id: VarInt,
    pub category: SoundCategory,
    pub entity_id: VarInt,
    pub volume: f32,
    pub pitch: f32,
    pub seed: i64,
}
