use crate::{Decode, Difficulty, Encode, Packet};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode, Packet)]
pub struct ChangeDifficultyC2s {
    pub difficulty: Difficulty,
}
