use std::borrow::Cow;

use valence_text::Text;

use crate::{Decode, Encode, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct ProfilelessChatMessageS2c<'a> {
    pub message: Cow<'a, Text>,
    pub chat_type: VarInt,
    pub chat_type_name: Cow<'a, Text>,
    pub target_name: Option<Cow<'a, Text>>,
}
