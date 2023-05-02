use crate::packet::raw::RawBytes;
use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct LoginQueryResponseC2s<'a> {
    pub message_id: VarInt,
    pub data: Option<RawBytes<'a>>,
}
