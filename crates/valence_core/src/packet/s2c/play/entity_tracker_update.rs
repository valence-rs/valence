use crate::packet::raw::RawBytes;
use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct EntityTrackerUpdateS2c<'a> {
    pub entity_id: VarInt,
    pub metadata: RawBytes<'a>,
}
