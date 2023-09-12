use valence_server::protocol::packets::play::command_tree_s2c::Parser;

use crate::parsers::{CommandArg, CommandArgParseError, ParseInput};

impl CommandArg for bool {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        if input.match_next("true") {
            Ok(true)
        } else if input.match_next("false") {
            Ok(false)
        } else {
            Err(CommandArgParseError::InvalidArgument(
                "bool".to_string(),
                input.input.clone(),
            ))
        }
    }

    fn display() -> Parser {
        Parser::Bool
    }
}

#[test]
fn test_bool() {
    let mut input = ParseInput::new("true".to_string());
    assert!(bool::parse_arg(&mut input).unwrap());
    assert!(input.is_done());

    let mut input = ParseInput::new("false".to_string());
    assert!(!bool::parse_arg(&mut input).unwrap());
    assert!(input.is_done());

    let mut input = ParseInput::new("false ".to_string());
    assert!(!bool::parse_arg(&mut input).unwrap());
    assert!(!input.is_done());

    let mut input = ParseInput::new("falSe trUe".to_string());
    assert!(!bool::parse_arg(&mut input).unwrap());
    assert!(!input.is_done());

    let mut input = ParseInput::new("fAlse true".to_string());
    assert!(!bool::parse_arg(&mut input).unwrap());
    assert!(!input.is_done());

    let mut input = ParseInput::new("false true".to_string());
    assert!(!bool::parse_arg(&mut input).unwrap());
    assert!(!input.is_done());

    let mut input = ParseInput::new("false true".to_string());
    assert!(!bool::parse_arg(&mut input).unwrap());
    assert!(!input.is_done());
}