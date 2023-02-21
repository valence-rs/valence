use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x12]
pub struct UpdateDifficultyLockC2s {
    pub locked: bool,
}
