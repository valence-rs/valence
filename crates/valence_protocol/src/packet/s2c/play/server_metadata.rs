use std::borrow::Cow;

use crate::text::Text;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct ServerMetadataS2c<'a> {
    pub motd: Cow<'a, Text>,
    pub icon: Option<&'a [u8]>,
    pub enforce_secure_chat: bool,
}
