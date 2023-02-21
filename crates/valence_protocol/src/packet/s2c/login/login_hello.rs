use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x01]
pub struct LoginHelloS2c<'a> {
    pub server_id: &'a str,
    pub public_key: &'a [u8],
    pub verify_token: &'a [u8],
}
