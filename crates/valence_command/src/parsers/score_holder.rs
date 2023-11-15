use super::Parser;
use crate::parsers::entity_selector::EntitySelector;
use crate::parsers::{CommandArg, CommandArgParseError, ParseInput};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ScoreHolder {
    Entity(EntitySelector),
    #[default]
    All,
}

impl CommandArg for ScoreHolder {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        if input.peek() == Some('*') {
            Ok(ScoreHolder::All)
        } else {
            Ok(ScoreHolder::Entity(EntitySelector::parse_arg(input)?))
        }
    }

    fn display() -> Parser {
        Parser::ScoreHolder {
            allow_multiple: false,
        }
    }
}
