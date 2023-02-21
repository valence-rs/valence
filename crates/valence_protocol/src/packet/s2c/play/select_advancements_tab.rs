use crate::ident::Ident;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x40]
pub struct SelectAdvancementsTabS2c<'a> {
    pub identifier: Option<Ident<&'a str>>,
}
