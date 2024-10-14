use crate::{Decode, Encode, Packet, VarInt};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct SetChunkCacheCenterS2c {
    pub chunk_x: VarInt,
    pub chunk_z: VarInt,
}
