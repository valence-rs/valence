use super::Parser;
use crate::parsers::{CommandArg, CommandArgParseError, ParseInput};

impl CommandArg for bool {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        if input.match_next("true") {
            Ok(true)
        } else if input.match_next("false") {
            Ok(false)
        } else {
            Err(CommandArgParseError::InvalidArgument {
                expected: "bool".to_string(),
                got: input.input.clone(),
            })
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
    assert!(bool::parse_arg(&mut input).unwrap());
    assert!(!input.is_done());
}
