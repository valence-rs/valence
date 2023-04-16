use std::borrow::Cow;

use crate::var_long::VarLong;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct ChunkDeltaUpdateS2c<'a> {
    pub chunk_section_position: i64,
    pub invert_trust_edges: bool,
    pub blocks: Cow<'a, [VarLong]>,
}
