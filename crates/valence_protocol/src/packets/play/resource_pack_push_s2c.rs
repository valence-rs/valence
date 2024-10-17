use std::borrow::Cow;

use uuid::Uuid;
use valence_text::Text;

use crate::{Bounded, Decode, Encode, Packet};

#[derive(Clone, PartialEq, Debug, Encode, Decode, Packet)]
pub struct ResourcePackPushS2c<'a> {
    pub uuid: Uuid,
    pub url: Bounded<&'a str, 32767>,
    pub hash: Bounded<&'a str, 40>,
    pub forced: bool,
    pub prompt_message: Option<Cow<'a, Text>>,
}
