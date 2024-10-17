use std::borrow::Cow;

use valence_text::Text;

use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct ServerDataS2c<'a> {
    pub motd: Cow<'a, Text>,
    pub icon: Option<&'a [u8]>,
}
