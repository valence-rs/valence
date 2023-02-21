use std::borrow::Cow;

use crate::text::Text;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x17]
pub struct DisconnectS2c<'a> {
    pub reason: Cow<'a, Text>,
}
