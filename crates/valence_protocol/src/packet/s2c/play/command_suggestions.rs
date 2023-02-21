use std::borrow::Cow;

use crate::text::Text;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x0d]
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
