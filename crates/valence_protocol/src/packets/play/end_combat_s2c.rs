use super::*;

/// Unused by notchian clients.
#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::END_COMBAT_S2C)]
pub struct EndCombatS2c {
    pub duration: VarInt,
}
