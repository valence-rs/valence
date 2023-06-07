use valence_core::protocol::packet::command::Parser;
use valence_core::translation_key::{
    ARGUMENTS_SWIZZLE_INVALID, ARGUMENT_ANGLE_INVALID, ARGUMENT_POS_MIXED,
    COMMAND_EXPECTED_SEPARATOR,
};

use crate::parser::{
    BrigadierArgument, Parse, ParsingBuild, ParsingError, ParsingPurpose, ParsingResult,
    ParsingSuggestions, Suggestion,
};
use crate::parsing_error;
use crate::primitive::ArrayError;
use crate::reader::StrReader;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RelativeValue<T> {
    Relative(T),
    Absolute(T),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RelativeValueError<E> {
    MixedPos,
    Value(E),
}

impl<E: ParsingBuild<ParsingError>> ParsingBuild<ParsingError> for RelativeValueError<E> {
    fn build(self) -> ParsingError {
        match self {
            Self::MixedPos => ParsingError::translate(ARGUMENT_POS_MIXED, vec![]),
            Self::Value(err) => err.build(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RelativeValueSuggestion<'a>(&'a str);

impl<'a> ParsingBuild<ParsingSuggestions<'a>> for RelativeValueSuggestion<'a> {
    fn build(self) -> ParsingSuggestions<'a> {
        ParsingSuggestions::Owned(if self.0.starts_with('~') {
            vec![self.0.into(), self.0.get('~'.len_utf8()..).unwrap().into()]
        } else {
            vec![
                {
                    let mut str = String::new();
                    str.push('~');
                    str.push_str(self.0);
                    str
                }
                .into(),
                self.0.into(),
            ]
        })
    }
}

impl<'a, T: Parse<'a>> Parse<'a> for RelativeValue<T> {
    type Data = T::Data;

    type Error = RelativeValueError<T::Error>;

    type Suggestions = RelativeValueSuggestion<'a>;

    fn parse(
        data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        purpose: ParsingPurpose,
    ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
        let begin = reader.cursor();

        let relative = match reader.peek_char() {
            Some('^') => {
                reader.next_char();
                Err(RelativeValueError::MixedPos)
            }
            Some('~') => {
                reader.next_char();
                Ok(true)
            }
            _ => Ok(false),
        };

        let t_result = T::parse(
            data,
            reader,
            match relative {
                Err(_) => ParsingPurpose::Suggestion,
                Ok(_) => purpose,
            },
        );

        let result = relative
            .map_err(|err| (begin..reader.cursor(), err))
            .and_then(|relative| {
                t_result
                    .result
                    .map(|v| {
                        v.map(|v| {
                            if relative {
                                Self::Relative(v)
                            } else {
                                Self::Absolute(v)
                            }
                        })
                    })
                    .map_err(|(pos, err)| (pos, RelativeValueError::Value(err)))
            });

        ParsingResult {
            suggestions: Some((
                begin..reader.cursor(),
                RelativeValueSuggestion(
                    reader
                        .str()
                        .get(
                            begin.bytes + if result.is_err() { 1 } else { 0 }
                                ..reader.cursor().bytes,
                        )
                        .unwrap(),
                ),
            )),
            result,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Angle(pub RelativeValue<f32>);

parsing_error!(AngleError {
    MixedPos = ARGUMENT_POS_MIXED,
    Invalid = ARGUMENT_ANGLE_INVALID,
});

impl<'a> Parse<'a> for Angle {
    type Data = ();

    type Error = AngleError;

    type Suggestions = RelativeValueSuggestion<'a>;

    fn parse(
        _data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        purpose: ParsingPurpose,
    ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
        let result = RelativeValue::parse(Some(&(Some(-180.0), Some(180.0))), reader, purpose);
        ParsingResult {
            suggestions: result.suggestions,
            result: result.result.map(|v| v.map(Self)).map_err(|(pos, err)| {
                (
                    pos,
                    match err {
                        RelativeValueError::MixedPos => AngleError::MixedPos,
                        RelativeValueError::Value(_) => AngleError::Invalid,
                    },
                )
            }),
        }
    }
}

impl<'a> BrigadierArgument<'a> for Angle {
    fn parser(_data: Option<&Self::Data>) -> Parser<'a> {
        Parser::Angle
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VectorA<const C: usize, T> {
    Relative([T; C]),
    Absolute([RelativeValue<T>; C]),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VectorASuggestions<'a> {
    Everything,
    Caret(&'a str),
    Absolute(&'a str),
}

const EVERYTHING_SUGGESTIONS: &[Suggestion<'static>] =
    &[Suggestion::new_str("~"), Suggestion::new_str("^")];

impl<'a> ParsingBuild<ParsingSuggestions<'a>> for VectorASuggestions<'a> {
    fn build(self) -> ParsingSuggestions<'a> {
        match self {
            Self::Everything => ParsingSuggestions::Borrowed(EVERYTHING_SUGGESTIONS),
            Self::Caret(given) => ParsingSuggestions::Owned(vec![{
                let mut str = String::new();
                str.push('^');
                str.push_str(given);
                str.into()
            }]),
            Self::Absolute(given) => RelativeValueSuggestion(given).build(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VectorAError<E> {
    MixedPos,
    Value(E),
    Separator,
}

impl<E: ParsingBuild<ParsingError>> ParsingBuild<ParsingError> for VectorAError<E> {
    fn build(self) -> ParsingError {
        match self {
            Self::MixedPos => RelativeValueError::<E>::MixedPos.build(),
            Self::Value(value) => RelativeValueError::Value(value).build(),
            Self::Separator => ParsingError::translate(COMMAND_EXPECTED_SEPARATOR, vec![]),
        }
    }
}

impl<E> From<RelativeValueError<E>> for VectorAError<E> {
    fn from(value: RelativeValueError<E>) -> Self {
        match value {
            RelativeValueError::MixedPos => Self::MixedPos,
            RelativeValueError::Value(e) => Self::Value(e),
        }
    }
}

impl<E> From<E> for VectorAError<E> {
    fn from(value: E) -> Self {
        Self::Value(value)
    }
}

impl<'a, const C: usize, T: Parse<'a> + Sized> Parse<'a> for VectorA<C, T>
where
    T::Data: Sized,
{
    type Data = [Option<T::Data>; C];

    type Error = VectorAError<T::Error>;

    type Suggestions = VectorASuggestions<'a>;

    fn parse(
        data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        purpose: ParsingPurpose,
    ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
        if reader.peek_char() == Some('^') {
            #[repr(transparent)]
            struct CaretValue<T>(T);

            impl<'a, T: Parse<'a> + Sized> Parse<'a> for CaretValue<T> {
                type Data = T::Data;

                type Error = VectorAError<T::Error>;

                type Suggestions = VectorASuggestions<'a>;

                fn parse(
                    data: Option<&Self::Data>,
                    reader: &mut StrReader<'a>,
                    purpose: ParsingPurpose,
                ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
                    let begin = reader.cursor();

                    if reader.next_char() != Some('^') {
                        return ParsingResult {
                            suggestions: Some((
                                begin..reader.cursor(),
                                VectorASuggestions::Caret(""),
                            )),
                            result: Err((begin..reader.cursor(), VectorAError::MixedPos)),
                        };
                    }

                    let t_result = T::parse(data, reader, purpose);

                    ParsingResult {
                        suggestions: None,
                        result: t_result
                            .result
                            .map(|v| v.map(|v| CaretValue(v)))
                            .map_err(|(pos, err)| (pos, err.into())),
                    }
                }
            }

            let result = <[CaretValue<T>; C]>::parse(data, reader, purpose);

            ParsingResult {
                suggestions: result.suggestions,
                result: result
                    .result
                    .map(|v| v.map(|v| Self::Relative(v.map(|v| v.0))))
                    .map_err(|(pos, err)| {
                        (
                            pos,
                            match err {
                                ArrayError::Separator => VectorAError::Separator,
                                ArrayError::Value(e) => e,
                            },
                        )
                    }),
            }
        } else {
            let result = <[RelativeValue<T>; C]>::parse(data, reader, purpose);

            ParsingResult {
                suggestions: result
                    .suggestions
                    .map(|(pos, s)| (pos, VectorASuggestions::Absolute(s.0))),
                result: result.result.map(|v| v.map(|v| Self::Absolute(v))).map_err(
                    |(pos, err)| {
                        (
                            pos,
                            match err {
                                ArrayError::Separator => VectorAError::Separator,
                                ArrayError::Value(e) => e.into(),
                            },
                        )
                    },
                ),
            }
        }
    }
}

pub type BlockPosArgument = VectorA<3, i32>;

impl<'a> BrigadierArgument<'a> for BlockPosArgument {
    fn parser(_data: Option<&Self::Data>) -> Parser<'a> {
        Parser::BlockPos
    }
}

pub type Vec3Argument = VectorA<3, f64>;

impl<'a> BrigadierArgument<'a> for Vec3Argument {
    fn parser(_data: Option<&Self::Data>) -> Parser<'a> {
        Parser::Vec3
    }
}

pub type ColumnPos = [RelativeValue<i32>; 2];

impl<'a> BrigadierArgument<'a> for ColumnPos {
    fn parser(_data: Option<&Self::Data>) -> Parser<'a> {
        Parser::ColumnPos
    }
}

pub type Vec2Argument = [RelativeValue<f64>; 2];

impl<'a> BrigadierArgument<'a> for Vec2Argument {
    fn parser(_data: Option<&Self::Data>) -> Parser<'a> {
        Parser::Vec2
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Swizzle([bool; 3]);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwizzleError;

impl ParsingBuild<ParsingError> for SwizzleError {
    fn build(self) -> ParsingError {
        ParsingError::translate(ARGUMENTS_SWIZZLE_INVALID, vec![])
    }
}

const SX: Suggestion<'static> = Suggestion::new_str("x");
const SY: Suggestion<'static> = Suggestion::new_str("y");
const SZ: Suggestion<'static> = Suggestion::new_str("z");

const S0: &[Suggestion<'static>] = &[SX, SY, SZ];
const S1: &[Suggestion<'static>] = &[SY, SZ];
const S2: &[Suggestion<'static>] = &[SX, SZ];
const S3: &[Suggestion<'static>] = &[SZ];
const S4: &[Suggestion<'static>] = &[SX, SY];
const S5: &[Suggestion<'static>] = &[SY];
const S6: &[Suggestion<'static>] = &[SX];
const S7: &[Suggestion<'static>] = &[];

const SWIZZLE_SUGGESTIONS: &[&[Suggestion<'static>]] = &[S0, S1, S2, S3, S4, S5, S6, S7];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwizzleSuggestions([bool; 3]);

impl<'a> ParsingBuild<ParsingSuggestions<'a>> for SwizzleSuggestions {
    fn build(self) -> ParsingSuggestions<'a> {
        ParsingSuggestions::Borrowed(
            SWIZZLE_SUGGESTIONS
                [self.0[0] as usize + self.0[1] as usize * 2 + self.0[2] as usize * 4],
        )
    }
}

impl<'a> Parse<'a> for Swizzle {
    type Data = ();

    type Error = SwizzleError;

    type Suggestions = SwizzleSuggestions;

    fn parse(
        _data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        _purpose: ParsingPurpose,
    ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
        let mut result = [false; 3];

        loop {
            let begin = reader.cursor();

            macro_rules! err {
                () => {{
                    return ParsingResult {
                        suggestions: Some((begin..reader.cursor(), SwizzleSuggestions(result))),
                        result: Err((begin..reader.cursor(), SwizzleError)),
                    };
                }};
            }

            let ch = reader.peek_char();
            let i = match ch {
                Some('x') => 0,
                Some('y') => 1,
                Some('z') => 2,
                Some(' ') | None => {
                    break;
                }
                Some(_) => err!(),
            };

            reader.next_char();

            if result[i] {
                err!();
            }

            result[i] = true;
        }

        ParsingResult {
            suggestions: Some((reader.cursor()..reader.cursor(), SwizzleSuggestions(result))),
            result: Ok(Some(Self(result))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reader::StrCursor;

    #[test]
    fn relative_value_test() {
        assert_eq!(
            RelativeValue::parse(None, &mut StrReader::new("~32"), ParsingPurpose::Reading),
            ParsingResult {
                suggestions: Some((
                    StrCursor::new_range("", "~32"),
                    RelativeValueSuggestion("~32")
                )),
                result: Ok(Some(RelativeValue::Relative(32))),
            }
        );

        assert_eq!(
            RelativeValue::parse(None, &mut StrReader::new("42"), ParsingPurpose::Reading),
            ParsingResult {
                suggestions: Some((
                    StrCursor::new_range("", "42"),
                    RelativeValueSuggestion("42")
                )),
                result: Ok(Some(RelativeValue::Absolute(42))),
            }
        );
    }

    #[test]
    fn vector_test() {
        assert_eq!(
            VectorA::parse(
                None,
                &mut StrReader::new("^32 ^32 ^90"),
                ParsingPurpose::Reading
            ),
            ParsingResult {
                suggestions: None,
                result: Ok(Some(VectorA::Relative([32, 32, 90])))
            }
        );

        assert_eq!(
            VectorA::parse(
                None,
                &mut StrReader::new("32 ~32 90"),
                ParsingPurpose::Reading
            ),
            ParsingResult {
                suggestions: None,
                result: Ok(Some(VectorA::Absolute([
                    RelativeValue::Absolute(32),
                    RelativeValue::Relative(32),
                    RelativeValue::Absolute(90)
                ])))
            }
        );

        assert_eq!(
            VectorA::<3, i32>::parse(
                None,
                &mut StrReader::new("32 ^32 90"),
                ParsingPurpose::Reading
            ),
            ParsingResult {
                suggestions: Some((
                    StrCursor::new_range("32 ", "^32"),
                    VectorASuggestions::Absolute("32")
                )),
                result: Err((StrCursor::new_range("32 ", "^32"), VectorAError::MixedPos)),
            }
        );
    }
}
