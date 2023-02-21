use crate::types::Hand;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x32]
pub struct PlayerInteractItemC2s {
    pub hand: Hand,
    pub sequence: VarInt,
}
