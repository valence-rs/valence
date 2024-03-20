use std::borrow::Cow;

use valence_text::Text;

use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct DisconnectS2c<'a> {
    pub reason: Cow<'a, Text>,
}
