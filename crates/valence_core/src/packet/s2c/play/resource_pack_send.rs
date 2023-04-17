use std::borrow::Cow;

use crate::packet::{Decode, Encode};
use crate::text::Text;

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct ResourcePackSendS2c<'a> {
    pub url: &'a str,
    pub hash: &'a str,
    pub forced: bool,
    pub prompt_message: Option<Cow<'a, Text>>,
}
