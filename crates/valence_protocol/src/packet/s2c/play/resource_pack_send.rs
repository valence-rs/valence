use std::borrow::Cow;

use crate::text::Text;
use crate::{Decode, Encode};

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct ResourcePackSendS2c<'a> {
    pub url: &'a str,
    pub hash: &'a str,
    pub forced: bool,
    pub prompt_message: Option<Cow<'a, Text>>,
}
