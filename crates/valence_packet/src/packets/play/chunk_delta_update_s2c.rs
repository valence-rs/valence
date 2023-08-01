use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CHUNK_DELTA_UPDATE_S2C)]
pub struct ChunkDeltaUpdateS2c<'a> {
    pub chunk_section_position: i64,
    pub blocks: Cow<'a, [VarLong]>,
}
