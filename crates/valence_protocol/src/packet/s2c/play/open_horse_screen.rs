use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct OpenHorseScreenS2c {
    pub window_id: u8,
    pub slot_count: VarInt,
    pub entity_id: i32,
}
