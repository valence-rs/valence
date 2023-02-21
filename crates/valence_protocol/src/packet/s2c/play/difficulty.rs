use crate::types::Difficulty;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x0b]
pub struct DifficultyS2c {
    pub difficulty: Difficulty,
    pub locked: bool,
}
