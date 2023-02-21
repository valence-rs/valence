use std::borrow::Cow;

use crate::text::Text;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x18]
pub struct ProfilelessChatMessageS2c<'a> {
    pub message: Cow<'a, Text>,
    pub chat_type: VarInt,
    pub chat_type_name: Cow<'a, Text>,
    pub target_name: Option<Cow<'a, Text>>,
}
