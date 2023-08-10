use super::*;

/// Unused by notchian clients.
#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::ENTER_COMBAT_S2C)]
pub struct EnterCombatS2c;
