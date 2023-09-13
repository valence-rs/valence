use valence_text::color::NamedColor;
use valence_text::Color;

use super::Parser;
use crate::parsers::{CommandArg, CommandArgParseError, ParseInput};

impl CommandArg for Color {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        if input.match_next("black") {
            Ok(Self::Named(NamedColor::Black))
        } else if input.match_next("dark_blue") {
            Ok(Self::Named(NamedColor::DarkBlue))
        } else if input.match_next("dark_green") {
            Ok(Self::Named(NamedColor::DarkGreen))
        } else if input.match_next("dark_aqua") {
            Ok(Self::Named(NamedColor::DarkAqua))
        } else if input.match_next("dark_red") {
            Ok(Self::Named(NamedColor::DarkRed))
        } else if input.match_next("dark_purple") {
            Ok(Self::Named(NamedColor::DarkPurple))
        } else if input.match_next("gold") {
            Ok(Self::Named(NamedColor::Gold))
        } else if input.match_next("gray") {
            Ok(Self::Named(NamedColor::Gray))
        } else if input.match_next("dark_gray") {
            Ok(Self::Named(NamedColor::DarkGray))
        } else if input.match_next("blue") {
            Ok(Self::Named(NamedColor::Blue))
        } else if input.match_next("green") {
            Ok(Self::Named(NamedColor::Green))
        } else if input.match_next("aqua") {
            Ok(Self::Named(NamedColor::Aqua))
        } else if input.match_next("red") {
            Ok(Self::Named(NamedColor::Red))
        } else if input.match_next("light_purple") {
            Ok(Self::Named(NamedColor::LightPurple))
        } else if input.match_next("yellow") {
            Ok(Self::Named(NamedColor::Yellow))
        } else if input.match_next("white") {
            Ok(Self::Named(NamedColor::White))
        } else if input.match_next("reset") {
            Ok(Self::Reset)
        } else {
            Err(CommandArgParseError::InvalidArgument(
                "chat_color".to_string(),
                "not a valid chat color".to_string(),
            ))
        }
    }

    fn display() -> Parser {
        Parser::Color
    }
}
