use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x13]
pub struct CooldownUpdateS2c {
    pub item_id: VarInt,
    pub cooldown_ticks: VarInt,
}
