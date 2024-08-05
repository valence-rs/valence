use valence_text::Color;

use super::Parser;
use crate::parsers::{CommandArg, CommandArgParseError, ParseInput};

impl CommandArg for Color {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        if input.match_next("black") {
            Ok(Self::BLACK)
        } else if input.match_next("dark_blue") {
            Ok(Self::DARK_BLUE)
        } else if input.match_next("dark_green") {
            Ok(Self::DARK_GREEN)
        } else if input.match_next("dark_aqua") {
            Ok(Self::DARK_AQUA)
        } else if input.match_next("dark_red") {
            Ok(Self::DARK_RED)
        } else if input.match_next("dark_purple") {
            Ok(Self::DARK_PURPLE)
        } else if input.match_next("gold") {
            Ok(Self::GOLD)
        } else if input.match_next("gray") {
            Ok(Self::GRAY)
        } else if input.match_next("dark_gray") {
            Ok(Self::DARK_GRAY)
        } else if input.match_next("blue") {
            Ok(Self::BLUE)
        } else if input.match_next("green") {
            Ok(Self::GREEN)
        } else if input.match_next("aqua") {
            Ok(Self::AQUA)
        } else if input.match_next("red") {
            Ok(Self::RED)
        } else if input.match_next("light_purple") {
            Ok(Self::LIGHT_PURPLE)
        } else if input.match_next("yellow") {
            Ok(Self::YELLOW)
        } else if input.match_next("white") {
            Ok(Self::WHITE)
        } else if input.match_next("reset") {
            Ok(Self::Reset)
        } else {
            Err(CommandArgParseError::InvalidArgument {
                expected: "chat_color".to_owned(),
                got: "not a valid chat color".to_owned(),
            })
        }
    }

    fn display() -> Parser {
        Parser::Color
    }
}
