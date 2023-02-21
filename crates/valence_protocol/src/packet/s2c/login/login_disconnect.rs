use std::borrow::Cow;

use crate::text::Text;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x00]
pub struct LoginDisconnectS2c<'a> {
    pub reason: Cow<'a, Text>,
}
