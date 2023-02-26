use std::borrow::Cow;

use crate::text::Text;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct SubtitleS2c<'a> {
    pub subtitle_text: Cow<'a, Text>,
}
