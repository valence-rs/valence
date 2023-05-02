use crate::hand::Hand;
use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct PlayerInteractItemC2s {
    pub hand: Hand,
    pub sequence: VarInt,
}
