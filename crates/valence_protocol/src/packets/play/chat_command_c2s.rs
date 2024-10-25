use std::borrow::Cow;

use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct ChatCommandC2s<'a> {
    pub command: Cow<'a, str>,
}
