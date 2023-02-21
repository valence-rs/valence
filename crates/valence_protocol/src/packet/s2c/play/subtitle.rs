use std::borrow::Cow;

use crate::text::Text;

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x59]
pub struct SubtitleS2c<'a> {
    pub subtitle_text: Cow<'a, Text>,
}
