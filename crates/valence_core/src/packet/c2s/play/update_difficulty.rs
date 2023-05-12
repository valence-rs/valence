use crate::difficulty::Difficulty;
use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub struct UpdateDifficultyC2s {
    pub difficulty: Difficulty,
}
