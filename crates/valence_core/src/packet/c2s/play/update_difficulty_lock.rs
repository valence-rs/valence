use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct UpdateDifficultyLockC2s {
    pub locked: bool,
}
