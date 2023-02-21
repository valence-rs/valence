use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x50]
pub struct EntityVelocityUpdateS2c {
    pub entity_id: VarInt,
    pub velocity: [i16; 3],
}
