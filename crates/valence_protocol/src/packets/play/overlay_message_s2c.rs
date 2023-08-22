use std::borrow::Cow;

use valence_text::Text;

use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct OverlayMessageS2c<'a> {
    pub action_bar_text: Cow<'a, Text>,
}
