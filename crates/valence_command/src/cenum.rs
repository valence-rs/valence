use std::marker::PhantomData;

use valence_core::game_mode::GameMode;
use valence_core::protocol::packet::command::Parser;
use valence_core::translation_key::{ARGUMENT_COLOR_INVALID, ARGUMENT_GAMEMODE_INVALID};

use crate::parser::{
    BrigadierArgument, Parsable, ParsingBuild, ParsingError, ParsingPurpose, ParsingResult,
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
/// - swizzle (?)
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

impl<'a, E: CEnum + 'a> Parsable<'a> for E {
    type Data = ();

    type Error = CEnumError<'a, E>;

    type Suggestions = CEnumSuggestions<E>;

    fn parse(
        _data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        _purpose: ParsingPurpose,
    ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
        let begin = reader.cursor();
        let str = reader.read_unquoted_str();

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
        impl CEnum for $name {
            const SUGGESTIONS: &'static [Suggestion<'static>] = &[
                $(Suggestion::new_str(casey::snake!(stringify!($val))),)*
            ];

            fn error(str: &str) -> ParsingError {
                ParsingError::translate($error, vec![str.to_string().into()])
            }

            fn from_str(str: &str) -> Option<Self> {
                match str {
                    $(casey::snake!(stringify!($val)) => Some(Self::$val),)*
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cenum_test() {
        assert_eq!(
            GameMode::parse(
                None,
                &mut StrReader::new("survival"),
                ParsingPurpose::Reading
            ),
            ParsingResult {
                suggestions: Some((0..8, CEnumSuggestions(PhantomData))),
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
                suggestions: Some((0..11, CEnumSuggestions(PhantomData))),
                result: Ok(Some(ColorArgument::DarkPurple)),
            }
        );
    }
}
