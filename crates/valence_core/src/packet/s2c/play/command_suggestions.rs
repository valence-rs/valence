use std::borrow::Cow;

use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};
use crate::text::Text;

#[derive(Clone, Debug, Encode, Decode)]
pub struct CommandSuggestionsS2c<'a> {
    pub id: VarInt,
    pub start: VarInt,
    pub length: VarInt,
    pub matches: Vec<Match<'a>>,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct Match<'a> {
    pub suggested_match: &'a str,
    pub tooltip: Option<Cow<'a, Text>>,
}
