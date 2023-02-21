use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x0d]
pub struct BookUpdateC2s<'a> {
    pub slot: VarInt,
    pub entries: Vec<&'a str>,
    pub title: Option<&'a str>,
}
