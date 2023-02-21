use crate::ident::Ident;
use crate::raw_bytes::RawBytes;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x04]
pub struct LoginQueryRequestS2c<'a> {
    pub message_id: VarInt,
    pub channel: Ident<&'a str>,
    pub data: RawBytes<'a>,
}
