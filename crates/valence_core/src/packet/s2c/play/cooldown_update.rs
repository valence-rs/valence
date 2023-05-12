use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct CooldownUpdateS2c {
    pub item_id: VarInt,
    pub cooldown_ticks: VarInt,
}
