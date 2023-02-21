use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x4b]
pub struct ChunkLoadDistanceS2c {
    pub view_distance: VarInt,
}
