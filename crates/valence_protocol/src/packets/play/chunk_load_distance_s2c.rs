use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CHUNK_LOAD_DISTANCE_S2C)]
pub struct ChunkLoadDistanceS2c {
    pub view_distance: VarInt,
}
