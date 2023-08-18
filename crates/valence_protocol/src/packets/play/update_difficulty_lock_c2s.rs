use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct UpdateDifficultyLockC2s {
    pub locked: bool,
}
