use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x0e]
pub struct QueryEntityNbtC2s {
    pub transaction_id: VarInt,
    pub entity_id: VarInt,
}
