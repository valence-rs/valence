use super::Parser;
use crate::parsers::{AbsoluteOrRelative, CommandArg, CommandArgParseError, ParseInput};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BlockPos {
    pub x: AbsoluteOrRelative<i32>,
    pub y: AbsoluteOrRelative<i32>,
    pub z: AbsoluteOrRelative<i32>,
}

impl CommandArg for BlockPos {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        let x = AbsoluteOrRelative::<i32>::parse_arg(input)?;
        input.skip_whitespace();
        let y = AbsoluteOrRelative::<i32>::parse_arg(input)?;
        input.skip_whitespace();
        let z = AbsoluteOrRelative::<i32>::parse_arg(input)?;

        Ok(BlockPos { x, y, z })
    }

    fn display() -> Parser {
        Parser::BlockPos
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_pos() {
        let mut input = ParseInput::new("~-1 2 3");
        assert_eq!(
            BlockPos::parse_arg(&mut input).unwrap(),
            BlockPos {
                x: AbsoluteOrRelative::Relative(-1),
                y: AbsoluteOrRelative::Absolute(2),
                z: AbsoluteOrRelative::Absolute(3)
            }
        );
        assert!(input.is_done());

        let mut input = ParseInput::new("-1 ~2 3 ");
        assert_eq!(
            BlockPos::parse_arg(&mut input).unwrap(),
            BlockPos {
                x: AbsoluteOrRelative::Absolute(-1),
                y: AbsoluteOrRelative::Relative(2),
                z: AbsoluteOrRelative::Absolute(3)
            }
        );
        assert!(!input.is_done());

        let mut input = ParseInput::new("-1 2 ~3 4");
        assert_eq!(
            BlockPos::parse_arg(&mut input).unwrap(),
            BlockPos {
                x: AbsoluteOrRelative::Absolute(-1),
                y: AbsoluteOrRelative::Absolute(2),
                z: AbsoluteOrRelative::Relative(3)
            }
        );
        assert!(!input.is_done());
    }
}
