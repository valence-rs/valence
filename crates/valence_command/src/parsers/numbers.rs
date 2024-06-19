use super::Parser;
use crate::parsers::{CommandArg, CommandArgParseError, ParseInput};
macro_rules! impl_parser_for_number {
    ($type:ty, $name:expr, $parser:ident) => {
        impl CommandArg for $type {
            fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
                input.skip_whitespace();
                let s = input.pop_word();

                let parsed = s.parse::<$type>();

                parsed.map_err(|_| CommandArgParseError::InvalidArgument {
                    expected: $name.to_owned(),
                    got: s.to_owned(),
                })
            }

            fn display() -> Parser {
                Parser::$parser {
                    min: None,
                    max: None,
                }
            }
        }
    };
}

impl_parser_for_number!(f32, "float", Float);
impl_parser_for_number!(f64, "double", Double);
impl_parser_for_number!(i32, "integer", Integer);
impl_parser_for_number!(i64, "long", Long);
impl_parser_for_number!(u32, "unsigned integer", Integer);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_number() {
        let mut input = ParseInput::new("1");
        assert_eq!(1, i32::parse_arg(&mut input).unwrap());
        assert!(input.is_done());

        let mut input = ParseInput::new("1");
        assert_eq!(1, i64::parse_arg(&mut input).unwrap());
        assert!(input.is_done());

        let mut input = ParseInput::new("1.0");
        assert_eq!(1.0, f32::parse_arg(&mut input).unwrap());
        assert!(input.is_done());

        let mut input = ParseInput::new("1.0");
        assert_eq!(1.0, f64::parse_arg(&mut input).unwrap());
        assert!(input.is_done());

        let mut input = ParseInput::new("3.40282347e+38 ");
        assert_eq!(f32::MAX, f32::parse_arg(&mut input).unwrap());
        assert!(!input.is_done());
    }
}
