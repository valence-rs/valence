use crate::ident::Ident;
use crate::raw_bytes::RawBytes;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct CustomPayloadC2s<'a> {
    pub channel: Ident<&'a str>,
    pub data: RawBytes<'a>,
}
