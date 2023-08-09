use super::*;

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::DIFFICULTY_S2C)]
pub struct DifficultyS2c {
    pub difficulty: Difficulty,
    pub locked: bool,
}
