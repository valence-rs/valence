use crate::types::SoundCategory;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct PlaySoundFromEntityS2c {
    pub id: VarInt,
    pub category: SoundCategory,
    pub entity_id: VarInt,
    pub volume: f32,
    pub pitch: f32,
    pub seed: i64,
}
