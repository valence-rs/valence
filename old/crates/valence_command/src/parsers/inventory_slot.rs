use bevy_derive::Deref;

use super::Parser;
use crate::parsers::{CommandArg, CommandArgParseError, ParseInput};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deref)]
pub struct InventorySlot(pub u32);

impl CommandArg for InventorySlot {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        let slot = u32::parse_arg(input)?;

        Ok(InventorySlot(slot))
    }

    fn display() -> Parser {
        Parser::ItemSlot
    }
}
