use crate::raw_bytes::RawBytes;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct EntityTrackerUpdateS2c<'a> {
    pub entity_id: VarInt,
    pub metadata: RawBytes<'a>,
}
