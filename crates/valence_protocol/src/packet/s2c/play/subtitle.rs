use std::borrow::Cow;

use crate::text::Text;
use crate::{Encode, Decode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct SubtitleS2c<'a> {
    pub subtitle_text: Cow<'a, Text>,
}
