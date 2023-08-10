use super::*;
use crate::sound::SoundCategory;

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
