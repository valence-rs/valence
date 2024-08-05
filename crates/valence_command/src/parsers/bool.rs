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
                expected: "bool".to_owned(),
                got: input.peek_word().to_owned(),
            })
        }
    }

    fn display() -> Parser {
        Parser::Bool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bool() {
        let mut input = ParseInput::new("true");
        assert!(bool::parse_arg(&mut input).unwrap());
        assert!(input.is_done());

        let mut input = ParseInput::new("false");
        assert!(!bool::parse_arg(&mut input).unwrap());
        assert!(input.is_done());

        let mut input = ParseInput::new("false ");
        assert!(!bool::parse_arg(&mut input).unwrap());
        assert!(!input.is_done());

        let mut input = ParseInput::new("falSe trUe");
        assert!(!bool::parse_arg(&mut input).unwrap());
        assert!(bool::parse_arg(&mut input).unwrap());
        assert!(input.is_done());
    }
}
