use valence_server::protocol::packets::play::command_tree_s2c::Parser;
use valence_server::GameMode;

use crate::parsers::{CommandArg, CommandArgParseError, ParseInput};

impl CommandArg for GameMode {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        if input.match_next("survival") {
            Ok(GameMode::Survival)
        } else if input.match_next("creative") {
            Ok(GameMode::Creative)
        } else if input.match_next("adventure") {
            Ok(GameMode::Adventure)
        } else if input.match_next("spectator") {
            Ok(GameMode::Spectator)
        } else {
            Err(CommandArgParseError::InvalidArgument(
                "game_mode".to_string(),
                "not a valid game mode".to_string(),
            ))
        }
    }

    fn display() -> Parser {
        Parser::GameMode
    }
}
