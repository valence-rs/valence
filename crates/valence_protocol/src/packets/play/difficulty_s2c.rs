use crate::{Decode, Difficulty, Encode, Packet};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode, Packet)]
pub struct DifficultyS2c {
    pub difficulty: Difficulty,
    pub locked: bool,
}
