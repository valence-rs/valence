use crate::{Decode, Encode, Packet};

/// Unused by notchian clients.
#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode, Packet)]
pub struct PlayerCombatEnterS2c;
