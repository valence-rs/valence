use std::borrow::Cow;

use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::{packet_id, Decode, Encode, Packet};

use crate::parse::Suggestion;

#[derive(Encode, Decode, Packet, Clone, Debug, PartialEq)]
#[packet(id = packet_id::COMMAND_SUGGESTIONS_S2C)]
pub struct CommandSuggestionsS2c<'a> {
    pub id: VarInt,
    pub start: VarInt,
    pub length: VarInt,
    pub matches: Cow<'a, [Suggestion<'a>]>,
}

#[derive(Encode, Debug, Packet, Clone, Decode, PartialEq)]
#[packet(id = packet_id::COMMAND_TREE_S2C)]
pub struct CommandTreeS2c<N> {
    pub nodes: N,
    pub root_index: VarInt,
}

#[derive(Encode, Debug, Copy, Clone, PartialEq)]
pub(crate) struct CommandTreeRawNodes<'a> {
    pub count: VarInt,
    pub bytes: &'a [u8],
}
