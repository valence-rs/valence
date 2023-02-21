use crate::ident::Ident;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x2f]
pub struct CraftFailedResponseS2c<'a> {
    pub window_id: u8,
    pub recipe: Ident<&'a str>,
}
