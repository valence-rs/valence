use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

/// Unused by notchian clients.
#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode)]
pub struct EndCombatS2c {
    pub duration: VarInt,
    pub entity_id: i32,
}
