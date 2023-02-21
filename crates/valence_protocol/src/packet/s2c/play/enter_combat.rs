use crate::{Decode, Encode};

/// Unused by notchian clients.
#[derive(Copy, Clone, PartialEq, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x33]
pub struct EnterCombatS2c;
