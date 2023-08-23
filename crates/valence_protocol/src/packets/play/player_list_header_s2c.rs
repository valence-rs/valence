use std::borrow::Cow;

use valence_text::Text;

use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct PlayerListHeaderS2c<'a> {
    pub header: Cow<'a, Text>,
    pub footer: Cow<'a, Text>,
}
