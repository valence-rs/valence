use valence_server::protocol::packets::play::command_tree_s2c::Parser;

use crate::parsers::{CommandArg, CommandArgParseError, ParseInput};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dimension {
    #[default]
    Overworld,
    Nether,
    End,
}

impl CommandArg for Dimension {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        if input.match_next("overworld") {
            Ok(Dimension::Overworld)
        } else if input.match_next("nether") {
            Ok(Dimension::Nether)
        } else if input.match_next("end") {
            Ok(Dimension::End)
        } else {
            Err(CommandArgParseError::InvalidArgument(
                "dimension".to_string(),
                "not a valid dimension".to_string(),
            ))
        }
    }

    fn display() -> Parser {
        Parser::Dimension
    }
}
