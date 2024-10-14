use crate::{Decode, Encode, Packet, VarInt};

/// Unused by notchian clients.
#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode, Packet)]
pub struct PlayerCombatEndS2c {
    pub duration: VarInt,
}
