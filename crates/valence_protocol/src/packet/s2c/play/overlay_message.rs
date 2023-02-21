use std::borrow::Cow;

use crate::text::Text;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x42]
pub struct OverlayMessageS2c<'a> {
    pub action_bar_text: Cow<'a, Text>,
}
