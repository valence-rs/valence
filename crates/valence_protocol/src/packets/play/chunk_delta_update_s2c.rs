use std::borrow::Cow;
use std::io::Write;

use bitfield_struct::bitfield;

use crate::{Decode, Encode, Packet, VarLong, chunk_section_pos::ChunkSectionPos};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct ChunkDeltaUpdateS2c<'a> {
    pub chunk_sect_pos: ChunkSectionPos,
    pub blocks: Cow<'a, [ChunkDeltaUpdateEntry]>,
}

#[bitfield(u64)]
#[derive(PartialEq, Eq)]
pub struct ChunkDeltaUpdateEntry {
    #[bits(4)]
    pub off_y: u8,
    #[bits(4)]
    pub off_z: u8,
    #[bits(4)]
    pub off_x: u8,
    pub block_state: u32,
    #[bits(20)]
    _pad: u32,
}

impl Encode for ChunkDeltaUpdateEntry {
    fn encode(&self, w: impl Write) -> anyhow::Result<()> {
        VarLong(self.0 as _).encode(w)
    }
}

impl Decode<'_> for ChunkDeltaUpdateEntry {
    fn decode(r: &mut &[u8]) -> anyhow::Result<Self> {
        Ok(ChunkDeltaUpdateEntry(VarLong::decode(r)?.0 as _))
    }
}
