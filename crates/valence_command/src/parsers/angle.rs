use bevy_derive::Deref;

use super::Parser;
use crate::parsers::{CommandArg, CommandArgParseError, ParseInput};

#[derive(Debug, Clone, Copy, PartialEq, Default, Deref)]
pub struct Angle(pub f32);

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
