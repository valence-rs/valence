use std::borrow::Cow;

use valence_text::Text;

use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct SetActionBarTextS2c<'a> {
    pub action_bar_text: Cow<'a, Text>,
}
