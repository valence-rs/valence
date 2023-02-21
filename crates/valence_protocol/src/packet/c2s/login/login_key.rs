use crate::{Decode, DecodePacket, Encode, EncodePacket};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x01]
pub struct LoginKeyC2s<'a> {
    pub shared_secret: &'a [u8],
    pub verify_token: &'a [u8],
}
