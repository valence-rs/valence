use super::Parser;
use crate::parsers::{CommandArg, CommandArgParseError, ParseInput};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Angle(f32);

impl CommandArg for Angle {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        let angle = f32::parse_arg(input)?;

        Ok(Angle(angle))
    }

    fn display() -> Parser {
        Parser::Angle
    }
}
