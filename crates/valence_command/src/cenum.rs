use std::marker::PhantomData;

use valence_core::game_mode::GameMode;
use valence_core::protocol::packet::command::Parser;
use valence_core::translation_key::{
    ARGUMENTS_OPERATION_INVALID, ARGUMENT_ANCHOR_INVALID, ARGUMENT_COLOR_INVALID,
    ARGUMENT_ENUM_INVALID, ARGUMENT_GAMEMODE_INVALID,
};

use crate::parser::{
    BrigadierArgument, Parse, ParsingBuild, ParsingError, ParsingPurpose, ParsingResult,
    ParsingSuggestions, Suggestion,
};
use crate::reader::StrReader;

/// Represent any enum that has no fields in its variants.
///
/// Brigadier's CEnums are:
/// - color
/// - dimension
/// - entity_anchor
/// - gamemode
/// - heightmap
/// - operation
/// - template mirror
/// - template rotation
pub trait CEnum: Sized {
    const SUGGESTIONS: &'static [Suggestion<'static>];

    fn error(str: &str) -> ParsingError;

    fn from_str(str: &str) -> Option<Self>;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CEnumError<'a, E>(&'a str, PhantomData<E>);

impl<'a, E: CEnum> ParsingBuild<ParsingError> for CEnumError<'a, E> {
    fn build(self) -> ParsingError {
        E::error(self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CEnumSuggestions<E>(PhantomData<E>);

impl<'a, E: CEnum> ParsingBuild<ParsingSuggestions<'a>> for CEnumSuggestions<E> {
    fn build(self) -> ParsingSuggestions<'a> {
        ParsingSuggestions::Borrowed(E::SUGGESTIONS)
    }
}

impl<'a, E: CEnum + 'a> Parse<'a> for E {
    type Data = ();

    type Error = CEnumError<'a, E>;

    type Suggestions = CEnumSuggestions<E>;

    fn parse(
        _data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        _purpose: ParsingPurpose,
    ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
        let begin = reader.cursor();
        let str = reader.read_delimitted_str();

        ParsingResult {
            suggestions: Some((begin..reader.cursor(), CEnumSuggestions(PhantomData))),
            result: match E::from_str(str) {
                Some(e) => Ok(Some(e)),
                None => Err((begin..reader.cursor(), CEnumError(str, PhantomData))),
            },
        }
    }
}

macro_rules! cenum {
    ($name: ty; $error: expr => {
        $($val: ident$(,)?)*
    }) => {
        cenum!($name; $error => {
            $($val = casey::snake!(stringify!($val)),)*
        });
    };
    ($name: ty; $error: expr => {
        $($val: ident = $s: expr$(,)?)*
    }) => {
        impl CEnum for $name {
            const SUGGESTIONS: &'static [Suggestion<'static>] = &[
                $(Suggestion::new_str($s),)*
            ];

            fn error(str: &str) -> ParsingError {
                ParsingError::translate($error, vec![str.to_string().into()])
            }

            fn from_str(str: &str) -> Option<Self> {
                match str {
                    $($s => Some(Self::$val),)*
                    _ => None,
                }
            }
        }
    }
}

cenum!(GameMode; ARGUMENT_GAMEMODE_INVALID => {
    Survival,
    Creative,
    Adventure,
    Spectator,
});

impl<'a> BrigadierArgument<'a> for GameMode {
    fn parser(_data: Option<&Self::Data>) -> Parser<'a> {
        Parser::GameMode
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorArgument {
    Black,
    DarkBlue,
    DarkGreen,
    DarkAqua,
    DarkRed,
    DarkPurple,
    Gold,
    Gray,
    DarkGray,
    Blue,
    Green,
    Aqua,
    Red,
    LightPurple,
    Yellow,
    White,
    Reset,
}

cenum!(ColorArgument; ARGUMENT_COLOR_INVALID => {
    Black,
    DarkBlue,
    DarkGreen,
    DarkAqua,
    DarkRed,
    DarkPurple,
    Gold,
    Gray,
    DarkGray,
    Blue,
    Green,
    Aqua,
    Red,
    LightPurple,
    Yellow,
    White,
    Reset,
});

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EntityAnchor {
    Eyes,
    Feet,
}

cenum!(EntityAnchor; ARGUMENT_ANCHOR_INVALID => {
    Eyes,
    Feet,
});

impl<'a> BrigadierArgument<'a> for EntityAnchor {
    fn parser(_data: Option<&Self::Data>) -> Parser<'a> {
        Parser::EntityAnchor
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Heightmap {
    WorldSurface,
    MotionBlocking,
    MotionBlockingNoLeaves,
    OceanFloor,
}

cenum!(Heightmap; ARGUMENT_ENUM_INVALID => {
    WorldSurface,
    MotionBlocking,
    MotionBlockingNoLeaves,
    OceanFloor,
});

impl<'a> BrigadierArgument<'a> for Heightmap {
    fn parser(_data: Option<&Self::Data>) -> Parser<'a> {
        Parser::Heightmap
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Operation {
    /// =
    Eq,
    /// +=
    Add,
    /// -=
    Sub,
    /// *=
    Mul,
    /// /=
    Div,
    /// %=
    Mod,
    /// ><
    Swap,
    /// <
    Min,
    /// >
    Max,
}

cenum!(Operation; ARGUMENTS_OPERATION_INVALID => {
    Eq = "=",
    Add = "+=",
    Sub = "-=",
    Mul = "*=",
    Div = "/=",
    Mod = "%=",
    Swap = "><",
    Min = "<",
    Max = ">"
});

impl<'a> BrigadierArgument<'a> for Operation {
    fn parser(_data: Option<&Self::Data>) -> Parser<'a> {
        Parser::Operation
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TemplateMirror {
    None,
    FrontBack,
    LeftRight,
}

cenum!(TemplateMirror; ARGUMENT_ENUM_INVALID => {
    None,
    FrontBack,
    LeftRight,
});

impl<'a> BrigadierArgument<'a> for TemplateMirror {
    fn parser(_data: Option<&Self::Data>) -> Parser<'a> {
        Parser::TemplateMirror
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TemplateRotation {
    None,
    Clockwise90,
    CounterClockwise90,
    Clockwise180,
}

cenum!(TemplateRotation; ARGUMENT_ENUM_INVALID => {
    None = "none",
    Clockwise90 = "clockwise_90",
    CounterClockwise90 = "counterclockwise_90",
    Clockwise180 = "180",
});

impl<'a> BrigadierArgument<'a> for TemplateRotation {
    fn parser(_data: Option<&Self::Data>) -> Parser<'a> {
        Parser::TemplateRotation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::StrCursor;

    #[test]
    fn cenum_test() {
        assert_eq!(
            GameMode::parse(
                None,
                &mut StrReader::new("survival"),
                ParsingPurpose::Reading
            ),
            ParsingResult {
                suggestions: Some((
                    StrCursor::new_range("", "survival"),
                    CEnumSuggestions(PhantomData)
                )),
                result: Ok(Some(GameMode::Survival))
            }
        );

        assert_eq!(
            ColorArgument::parse(
                None,
                &mut StrReader::new("dark_purple"),
                ParsingPurpose::Reading
            ),
            ParsingResult {
                suggestions: Some((
                    StrCursor::new_range("", "dark_purple"),
                    CEnumSuggestions(PhantomData)
                )),
                result: Ok(Some(ColorArgument::DarkPurple)),
            }
        );
    }
}
