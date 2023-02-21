use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x1b]
pub struct UnloadChunkS2c {
    pub chunk_x: i32,
    pub chunk_z: i32,
}
