use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::UPDATE_DIFFICULTY_LOCK_C2S)]
pub struct UpdateDifficultyLockC2s {
    pub locked: bool,
}
