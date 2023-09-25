use crate::{Decode, Encode, Packet, VarInt};

/// Unused by notchian clients.
#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode, Packet)]
pub struct EndCombatS2c {
    pub duration: VarInt,
}
