use std::borrow::Cow;

use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x55]
pub struct EntityPassengersSetS2c {
    /// Vehicle's entity id
    pub entity_id: VarInt,
    pub passengers: Vec<VarInt>,
}
