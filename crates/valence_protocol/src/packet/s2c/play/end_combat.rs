use crate::var_int::VarInt;
use crate::{Decode, Encode};

/// Unused by notchian clients.
#[derive(Copy, Clone, PartialEq, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x32]
pub struct EndCombatS2c {
    pub duration: VarInt,
    pub entity_id: i32,
}
