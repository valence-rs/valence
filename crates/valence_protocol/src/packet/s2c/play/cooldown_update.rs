use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct CooldownUpdateS2c {
    pub item_id: VarInt,
    pub cooldown_ticks: VarInt,
}
