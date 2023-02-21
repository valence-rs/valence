use std::borrow::Cow;

use crate::text::Text;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x5b]
pub struct TitleS2c<'a> {
    pub title_text: Cow<'a, Text>,
}
