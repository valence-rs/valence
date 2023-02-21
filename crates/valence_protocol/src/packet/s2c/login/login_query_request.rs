use crate::ident::Ident;
use crate::raw_bytes::RawBytes;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct LoginQueryRequestS2c<'a> {
    pub message_id: VarInt,
    pub channel: Ident<&'a str>,
    pub data: RawBytes<'a>,
}
