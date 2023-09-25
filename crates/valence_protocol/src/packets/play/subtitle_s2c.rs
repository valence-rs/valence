use std::borrow::Cow;

use valence_text::Text;

use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct SubtitleS2c<'a> {
    pub subtitle_text: Cow<'a, Text>,
}
