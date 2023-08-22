use std::borrow::Cow;

use crate::{Decode, Encode, Packet, VarLong};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct ChunkDeltaUpdateS2c<'a> {
    pub chunk_section_position: i64,
    pub blocks: Cow<'a, [VarLong]>,
}
