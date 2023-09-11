use valence_server::protocol::packets::play::command_tree_s2c::Parser;

use crate::parsers::{AbsoluteOrRelative, CommandArg, CommandArgParseError, ParseInput};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    pub x: AbsoluteOrRelative<f32>,
    pub y: AbsoluteOrRelative<f32>,
}

impl CommandArg for Vec2 {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        let x = AbsoluteOrRelative::<f32>::parse_arg(input)?;
        input.skip_whitespace();
        let y = AbsoluteOrRelative::<f32>::parse_arg(input)?;

        Ok(Vec2 { x, y })
    }

    fn display() -> Parser {
        Parser::Vec2
    }
}

#[test]
fn test_vec2() {
    let mut input = ParseInput::new("~-1.5 2.5".to_string());
    assert_eq!(
        Vec2::parse_arg(&mut input).unwrap(),
        Vec2 {
            x: AbsoluteOrRelative::Relative(-1.5),
            y: AbsoluteOrRelative::Absolute(2.5),
        }
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("-1.5 ~2.5 ".to_string());
    assert_eq!(
        Vec2::parse_arg(&mut input).unwrap(),
        Vec2 {
            x: AbsoluteOrRelative::Absolute(-1.5),
            y: AbsoluteOrRelative::Relative(2.5),
        }
    );
    assert!(!input.is_done());

    let mut input = ParseInput::new("-1.5 2.5 3.5".to_string());
    assert_eq!(
        Vec2::parse_arg(&mut input).unwrap(),
        Vec2 {
            x: AbsoluteOrRelative::Absolute(-1.5),
            y: AbsoluteOrRelative::Absolute(2.5),
        }
    );
    assert!(!input.is_done());
}
