use crate::packet::{Decode, Encode};

/// Unused by notchian clients.
#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode)]
pub struct EnterCombatS2c;
