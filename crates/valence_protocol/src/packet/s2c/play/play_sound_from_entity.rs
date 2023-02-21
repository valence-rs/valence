use crate::types::SoundCategory;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x5d]
pub struct PlaySoundFromEntityS2c {
    pub id: VarInt,
    pub category: SoundCategory,
    pub entity_id: VarInt,
    pub volume: f32,
    pub pitch: f32,
    pub seed: i64,
}
