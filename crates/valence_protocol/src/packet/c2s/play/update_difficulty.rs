use crate::types::Difficulty;
use crate::{Decode, Encode};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
#[packet_id = 0x02]
pub struct UpdateDifficultyC2s {
    pub difficulty: Difficulty,
}
