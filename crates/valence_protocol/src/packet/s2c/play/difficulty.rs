use crate::types::Difficulty;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct DifficultyS2c {
    pub difficulty: Difficulty,
    pub locked: bool,
}
