use crate::ident::Ident;
use crate::raw_bytes::RawBytes;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x0c]
pub struct CustomPayloadC2s<'a> {
    pub channel: Ident<&'a str>,
    pub data: RawBytes<'a>,
}
