use super::*;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::UPDATE_DIFFICULTY_C2S)]
pub struct UpdateDifficultyC2s {
    pub difficulty: Difficulty,
}
