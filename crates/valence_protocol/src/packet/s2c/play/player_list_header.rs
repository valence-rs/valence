use std::borrow::Cow;

use crate::text::Text;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x61]
pub struct PlayerListHeaderS2c<'a> {
    pub header: Cow<'a, Text>,
    pub footer: Cow<'a, Text>,
}
