use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::COMMAND_SUGGESTIONS_S2C)]
pub struct CommandSuggestionsS2c<'a> {
    pub id: VarInt,
    pub start: VarInt,
    pub length: VarInt,
    pub matches: Vec<CommandSuggestionsMatch<'a>>,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub struct CommandSuggestionsMatch<'a> {
    pub suggested_match: &'a str,
    pub tooltip: Option<Cow<'a, Text>>,
}
