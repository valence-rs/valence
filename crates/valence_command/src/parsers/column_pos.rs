use super::Parser;
use crate::parsers::{AbsoluteOrRelative, CommandArg, CommandArgParseError, ParseInput};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ColumnPos {
    pub x: AbsoluteOrRelative<i32>,
    pub y: AbsoluteOrRelative<i32>,
    pub z: AbsoluteOrRelative<i32>,
}

impl CommandArg for ColumnPos {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        let x = AbsoluteOrRelative::<i32>::parse_arg(input)?;
        input.skip_whitespace();
        let y = AbsoluteOrRelative::<i32>::parse_arg(input)?;
        input.skip_whitespace();
        let z = AbsoluteOrRelative::<i32>::parse_arg(input)?;

        Ok(ColumnPos { x, y, z })
    }

    fn display() -> Parser {
        Parser::ColumnPos
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_pos() {
        let mut input = ParseInput::new("~-1 2 3");
        assert_eq!(
            ColumnPos::parse_arg(&mut input).unwrap(),
            ColumnPos {
                x: AbsoluteOrRelative::Relative(-1),
                y: AbsoluteOrRelative::Absolute(2),
                z: AbsoluteOrRelative::Absolute(3)
            }
        );
        assert!(input.is_done());

        let mut input = ParseInput::new("-1 ~2 3 hello");
        assert_eq!(
            ColumnPos::parse_arg(&mut input).unwrap(),
            ColumnPos {
                x: AbsoluteOrRelative::Absolute(-1),
                y: AbsoluteOrRelative::Relative(2),
                z: AbsoluteOrRelative::Absolute(3)
            }
        );
        assert!(!input.is_done());
        input.skip_whitespace();
        assert!(input.match_next("hello"));

        let mut input = ParseInput::new("-1 2 ~3 4");
        assert_eq!(
            ColumnPos::parse_arg(&mut input).unwrap(),
            ColumnPos {
                x: AbsoluteOrRelative::Absolute(-1),
                y: AbsoluteOrRelative::Absolute(2),
                z: AbsoluteOrRelative::Relative(3)
            }
        );
        assert!(!input.is_done());
    }
}
