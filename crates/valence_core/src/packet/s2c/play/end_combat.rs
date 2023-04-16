use crate::var_int::VarInt;
use crate::{Decode, Encode};

/// Unused by notchian clients.
#[derive(Copy, Clone, PartialEq, Debug, Encode, Decode)]
pub struct EndCombatS2c {
    pub duration: VarInt,
    pub entity_id: i32,
}
