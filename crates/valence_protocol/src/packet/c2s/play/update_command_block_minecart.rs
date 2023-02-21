use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x2a]
pub struct UpdateCommandBlockMinecartC2s<'a> {
    pub entity_id: VarInt,
    pub command: &'a str,
    pub track_output: bool,
}
