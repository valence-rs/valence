use valence_server::protocol::packets::play::command_tree_s2c::Parser;

use crate::parsers::{AbsoluteOrRelative, CommandArg, CommandArgParseError, ParseInput};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    pub x: AbsoluteOrRelative<f32>,
    pub y: AbsoluteOrRelative<f32>,
    pub z: AbsoluteOrRelative<f32>,
}

impl CommandArg for Vec3 {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        let x = AbsoluteOrRelative::<f32>::parse_arg(input)?;
        input.skip_whitespace();
        let y = AbsoluteOrRelative::<f32>::parse_arg(input)?;
        input.skip_whitespace();
        let z = AbsoluteOrRelative::<f32>::parse_arg(input)?;

        Ok(Vec3 { x, y, z })
    }

    fn display() -> Parser {
        Parser::Vec3
    }
}

#[test]
fn test_vec3() {
    let mut input = ParseInput::new("~-1.5 2.5 3.5".to_string());
    assert_eq!(
        Vec3::parse_arg(&mut input).unwrap(),
        Vec3 {
            x: AbsoluteOrRelative::Relative(-1.5),
            y: AbsoluteOrRelative::Absolute(2.5),
            z: AbsoluteOrRelative::Absolute(3.5)
        }
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("-1.5 ~2.5 3.5 ".to_string());
    assert_eq!(
        Vec3::parse_arg(&mut input).unwrap(),
        Vec3 {
            x: AbsoluteOrRelative::Absolute(-1.5),
            y: AbsoluteOrRelative::Relative(2.5),
            z: AbsoluteOrRelative::Absolute(3.5)
        }
    );
    assert!(!input.is_done());

    let mut input = ParseInput::new("-1.5 2.5 ~3.5 4.5".to_string());
    assert_eq!(
        Vec3::parse_arg(&mut input).unwrap(),
        Vec3 {
            x: AbsoluteOrRelative::Absolute(-1.5),
            y: AbsoluteOrRelative::Absolute(2.5),
            z: AbsoluteOrRelative::Relative(3.5)
        }
    );
    assert!(!input.is_done());
}