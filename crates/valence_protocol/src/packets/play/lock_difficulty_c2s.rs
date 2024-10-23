use crate::{Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct LockDifficultyC2s {
    pub locked: bool,
}
