use std::borrow::Cow;

use valence_text::Text;

use crate::{Bounded, Decode, Encode, Packet};

#[derive(Clone, PartialEq, Debug, Encode, Decode, Packet)]
pub struct ResourcePackSendS2c<'a> {
    pub url: &'a str,
    pub hash: Bounded<&'a str, 40>,
    pub forced: bool,
    pub prompt_message: Option<Cow<'a, Text>>,
}
