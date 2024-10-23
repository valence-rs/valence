use std::borrow::Cow;

use valence_text::Text;

use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct SetTitleTextS2c<'a> {
    pub title_text: Cow<'a, Text>,
}
