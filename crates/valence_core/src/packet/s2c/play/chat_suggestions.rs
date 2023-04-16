use std::borrow::Cow;

use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct ChatSuggestionsS2c<'a> {
    pub action: Action,
    pub entries: Cow<'a, [&'a str]>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum Action {
    Add,
    Remove,
    Set,
}
