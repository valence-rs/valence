use std::borrow::Cow;

use valence_core::protocol::packet::command::Node;
use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::{packet_id, Decode, Encode, Packet};

use crate::parse::Suggestion;

#[derive(Encode, Decode, Packet, Clone, Debug, PartialEq)]
#[packet(id = packet_id::REQUEST_COMMAND_COMPLETIONS_C2S)]
pub struct RequestCommandCompletionsC2s<'a> {
    pub id: VarInt,
    pub text: &'a str,
}

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

#[derive(Debug, Clone)]
pub(crate) struct CommandTreeRawNodes<'a> {
    pub count: VarInt,
    pub root: Node<'a>,
    pub bytes: &'a [u8],
}

impl<'a> Encode for CommandTreeRawNodes<'a> {
    fn encode(&self, mut w: impl std::io::Write) -> anyhow::Result<()> {
        self.count.encode(&mut w)?;
        self.root.encode(&mut w)?;
        w.write_all(self.bytes)?;
        Ok(())
    }
}
