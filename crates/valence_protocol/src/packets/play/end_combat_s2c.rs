use super::*;

/// Unused by notchian clients.
#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode, Packet)]
pub struct EndCombatS2c {
    pub duration: VarInt,
}
