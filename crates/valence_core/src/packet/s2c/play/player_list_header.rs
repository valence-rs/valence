use std::borrow::Cow;

use crate::packet::{Decode, Encode};
use crate::text::Text;

#[derive(Clone, Debug, Encode, Decode)]
pub struct PlayerListHeaderS2c<'a> {
    pub header: Cow<'a, Text>,
    pub footer: Cow<'a, Text>,
}
