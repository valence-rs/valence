use valence_server::protocol::packets::play::command_tree_s2c::Parser;

use crate::parsers::{CommandArg, CommandArgParseError, ParseInput};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EntityAnchor {
    #[default]
    Eyes,
    Feet,
}

impl CommandArg for EntityAnchor {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        if input.match_next("eyes") {
            Ok(EntityAnchor::Eyes)
        } else if input.match_next("feet") {
            Ok(EntityAnchor::Feet)
        } else {
            Err(CommandArgParseError::InvalidArgument(
                "entity_anchor".to_string(),
                "not a valid entity anchor".to_string(),
            ))
        }
    }

    fn display() -> Parser {
        Parser::EntityAnchor
    }
}
