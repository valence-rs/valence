use super::Parser;
use crate::parsers::vec2::Vec2;
use crate::parsers::{CommandArg, CommandArgParseError, ParseInput};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rotation(Vec2);

impl CommandArg for Rotation {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        let vec2 = Vec2::parse_arg(input)?;

        Ok(Rotation(vec2))
    }

    fn display() -> Parser {
        Parser::Rotation
    }
}
