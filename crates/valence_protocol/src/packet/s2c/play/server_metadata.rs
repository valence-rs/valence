use std::borrow::Cow;

use crate::text::Text;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct ServerMetadataS2c<'a> {
    pub motd: Option<Cow<'a, Text>>,
    pub icon: Option<&'a str>,
    pub enforce_secure_chat: bool,
}
