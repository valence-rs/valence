use super::Parser;
use crate::parsers::{CommandArg, CommandArgParseError, ParseInput};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Swizzle {
    pub x: bool,
    pub y: bool,
    pub z: bool,
}

impl CommandArg for Swizzle {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        let mut swizzle = Swizzle::default();
        while let Some(c) = input.peek() {
            match c {
                'x' => swizzle.x = true,
                'y' => swizzle.y = true,
                'z' => swizzle.z = true,
                _ => break,
            }
            input.pop();
        }

        Ok(swizzle)
    }

    fn display() -> Parser {
        Parser::Swizzle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swizzle() {
        let mut input = ParseInput::new("xyzzzz");
        let swizzle = Swizzle::parse_arg(&mut input).unwrap();
        assert_eq!(
            swizzle,
            Swizzle {
                x: true,
                y: true,
                z: true
            }
        );
        assert!(input.is_done());

        let mut input = ParseInput::new("xzy");
        let swizzle = Swizzle::parse_arg(&mut input).unwrap();
        assert_eq!(
            swizzle,
            Swizzle {
                x: true,
                y: true,
                z: true
            }
        );
        assert!(input.is_done());

        let mut input = ParseInput::new("x");
        let swizzle = Swizzle::parse_arg(&mut input).unwrap();
        assert_eq!(
            swizzle,
            Swizzle {
                x: true,
                y: false,
                z: false
            }
        );
        assert!(input.is_done());

        let mut input = ParseInput::new("x y z zy xyz");
        let swizzle_a = Swizzle::parse_arg(&mut input).unwrap();
        let swizzle_b = Swizzle::parse_arg(&mut input).unwrap();
        let swizzle_c = Swizzle::parse_arg(&mut input).unwrap();
        let swizzle_d = Swizzle::parse_arg(&mut input).unwrap();
        let swizzle_e = Swizzle::parse_arg(&mut input).unwrap();
        assert_eq!(
            swizzle_a,
            Swizzle {
                x: true,
                y: false,
                z: false
            }
        );
        assert_eq!(
            swizzle_b,
            Swizzle {
                x: false,
                y: true,
                z: false
            }
        );
        assert_eq!(
            swizzle_c,
            Swizzle {
                x: false,
                y: false,
                z: true
            }
        );
        assert_eq!(
            swizzle_d,
            Swizzle {
                x: false,
                y: true,
                z: true
            }
        );
        assert_eq!(
            swizzle_e,
            Swizzle {
                x: true,
                y: true,
                z: true
            }
        );
    }
}
