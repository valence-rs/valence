use std::borrow::Cow;
use std::mem::MaybeUninit;

use valence_core::protocol::packet::command::{Parser, StringArg};
use valence_core::translation_key::{
    ARGUMENT_DOUBLE_BIG, ARGUMENT_DOUBLE_LOW, ARGUMENT_FLOAT_BIG, ARGUMENT_FLOAT_LOW,
    ARGUMENT_INTEGER_BIG, ARGUMENT_INTEGER_LOW, ARGUMENT_LONG_BIG, ARGUMENT_LONG_LOW,
    COMMAND_EXPECTED_SEPARATOR, PARSING_BOOL_EXPECTED, PARSING_BOOL_INVALID,
    PARSING_DOUBLE_EXPECTED, PARSING_DOUBLE_INVALID, PARSING_FLOAT_EXPECTED, PARSING_FLOAT_INVALID,
    PARSING_INT_EXPECTED, PARSING_INT_INVALID, PARSING_LONG_EXPECTED, PARSING_LONG_INVALID,
    PARSING_QUOTE_EXPECTED_END,
};

use crate::parser::{
    BrigadierArgument, NoParsingBuild, Parse, ParsingBuild, ParsingError, ParsingPurpose,
    ParsingResult, ParsingSuggestions, Suggestion,
};
use crate::parsing_error;
use crate::reader::StrReader;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BoolParsingError<'a> {
    Expected,
    Invalid(&'a str),
}

impl<'a> ParsingBuild<ParsingError> for BoolParsingError<'a> {
    fn build(self) -> ParsingError {
        match self {
            Self::Expected => ParsingError::translate(PARSING_BOOL_EXPECTED, vec![]),
            Self::Invalid(given) => {
                ParsingError::translate(PARSING_BOOL_INVALID, vec![given.to_string().into()])
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoolSuggestions;

const BOOL_SUGGESTIONS: &[Suggestion<'static>] =
    &[Suggestion::new_str("true"), Suggestion::new_str("false")];

impl<'a> ParsingBuild<ParsingSuggestions<'a>> for BoolSuggestions {
    fn build(self) -> ParsingSuggestions<'a> {
        ParsingSuggestions::Borrowed(BOOL_SUGGESTIONS)
    }
}

impl<'a> Parse<'a> for bool {
    type Error = BoolParsingError<'a>;

    type Suggestions = BoolSuggestions;

    type Data = ();

    fn parse(
        _data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        _purpose: ParsingPurpose,
    ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
        let begin = reader.cursor();

        let result = match reader.read_unquoted_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            "" if reader.is_ended() => Err(BoolParsingError::Expected),
            o => Err(BoolParsingError::Invalid(o)),
        };

        let pos = begin..reader.cursor();

        ParsingResult {
            suggestions: Some((pos.clone(), BoolSuggestions)),
            result: result.map(Some).map_err(|err| (pos, err)),
        }
    }
}

impl<'a> BrigadierArgument<'a> for bool {
    fn parser(_data: Option<&Self::Data>) -> Parser<'a> {
        Parser::Bool
    }
}

macro_rules! num_impl {
    ($($ty:ty, $int: expr, $error_n:ident, $parser: ident, $too_big:expr, $too_low:expr, $expected:expr, $invalid:expr,)*) => {
        $(#[derive(Clone, Copy, Debug, PartialEq)]
        pub enum $error_n<'a> {
            TooBig(&'a str, $ty),
            TooLow(&'a str, $ty),
            Invalid(&'a str),
            Expected,
        }

        impl<'a> ParsingBuild<ParsingError> for $error_n<'a> {
            fn build(self) -> ParsingError {
                match self {
                    Self::TooBig(given, bound) => {
                        ParsingError::translate($too_big, vec![bound.to_string().into(), given.to_string().into()])
                    }
                    Self::TooLow(given, bound) => {
                        ParsingError::translate($too_low, vec![bound.to_string().into(), given.to_string().into()])
                    }
                    Self::Invalid(given) => {
                        ParsingError::translate($invalid, vec![given.to_string().into()])
                    }
                    Self::Expected => ParsingError::translate($expected, vec![])
                }
            }
        }

        impl<'a> Parse<'a> for $ty {
            type Error = $error_n<'a>;

            type Suggestions = NoParsingBuild;

            type Data = (Option<Self>, Option<Self>);

            fn parse(
                data: Option<&Self::Data>,
                reader: &mut StrReader<'a>,
                _purpose: ParsingPurpose,
            ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
                let begin = reader.cursor();

                let num_str = if $int {
                    reader.read_int_str()
                } else {
                    reader.read_float_str().0
                };

                let result = match (num_str.parse::<Self>(), data) {
                    (Ok(i), Some((Some(min), _))) if *min > i => {
                        Err($error_n::TooLow(num_str, *min))
                    },
                    (Ok(i), Some((_, Some(max)))) if *max < i => {
                        Err($error_n::TooBig(num_str, *max))
                    },
                    (Ok(i), _) => Ok(i),
                    (Err(_), _) if num_str.is_empty() && reader.is_ended() => Err($error_n::Expected),
                    (Err(_), _) => Err($error_n::Invalid(num_str))
                };

                ParsingResult {
                    suggestions: None,
                    result: result.map(|v| Some(v)).map_err(|err| (begin..reader.cursor(), err))
                }
            }
        }

        impl<'a> BrigadierArgument<'a> for $ty {
            fn parser(data: Option<&Self::Data>) -> Parser<'a> {
                let data = data.unwrap_or(&(None, None));
                Parser::$parser {
                    min: data.0,
                    max: data.1,
                }
            }
        })*
    };
}

num_impl!(
    i32,
    true,
    I32Error,
    Integer,
    ARGUMENT_INTEGER_BIG,
    ARGUMENT_INTEGER_LOW,
    PARSING_INT_EXPECTED,
    PARSING_INT_INVALID,
    i64,
    true,
    I64Error,
    Long,
    ARGUMENT_LONG_BIG,
    ARGUMENT_LONG_LOW,
    PARSING_LONG_EXPECTED,
    PARSING_LONG_INVALID,
    f32,
    false,
    F32Error,
    Float,
    ARGUMENT_FLOAT_BIG,
    ARGUMENT_FLOAT_LOW,
    PARSING_FLOAT_EXPECTED,
    PARSING_FLOAT_INVALID,
    f64,
    false,
    F64Error,
    Double,
    ARGUMENT_DOUBLE_BIG,
    ARGUMENT_DOUBLE_LOW,
    PARSING_DOUBLE_EXPECTED,
    PARSING_DOUBLE_INVALID,
);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SingleWordString<'a>(pub &'a str);

impl<'a> Parse<'a> for SingleWordString<'a> {
    type Data = ();

    type Error = NoParsingBuild;

    type Suggestions = NoParsingBuild;

    fn parse(
        _data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        _purpose: ParsingPurpose,
    ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
        ParsingResult {
            suggestions: None,
            result: Ok(Some(SingleWordString(reader.read_unquoted_str()))),
        }
    }
}

impl<'a> BrigadierArgument<'a> for SingleWordString<'a> {
    fn parser(_data: Option<&Self::Data>) -> Parser<'a> {
        Parser::String(StringArg::SingleWord)
    }
}

parsing_error!(UnclosedQuoteError = PARSING_QUOTE_EXPECTED_END);

#[derive(Clone, Debug, PartialEq)]
pub struct QuotableString<'a>(pub Cow<'a, str>);

impl<'a> Parse<'a> for QuotableString<'a> {
    type Data = ();

    type Error = UnclosedQuoteError;

    type Suggestions = NoParsingBuild;

    fn parse(
        _data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        purpose: ParsingPurpose,
    ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
        let begin = reader.cursor();

        let quoted = if reader.peek_char() == Some('"') {
            reader.next_char();
            true
        } else {
            false
        };

        let result = match purpose {
            ParsingPurpose::Reading => {
                if quoted {
                    reader
                        .read_started_quoted_str()
                        .map(|v| Some(Self(Cow::Owned(v))))
                        .ok_or(UnclosedQuoteError)
                } else {
                    Ok(Some(Self(Cow::Borrowed(reader.read_unquoted_str()))))
                }
            }
            ParsingPurpose::Suggestion => {
                if quoted {
                    if reader.skip_started_quoted_str() {
                        Ok(None)
                    } else {
                        Err(UnclosedQuoteError)
                    }
                } else {
                    reader.read_unquoted_str();
                    Ok(None)
                }
            }
        };

        ParsingResult {
            suggestions: None,
            result: result.map_err(|err| (begin..reader.cursor(), err)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GreedyString<'a>(pub &'a str);

impl<'a> Parse<'a> for GreedyString<'a> {
    type Data = ();

    type Error = NoParsingBuild;

    type Suggestions = NoParsingBuild;

    fn parse(
        _data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        _purpose: ParsingPurpose,
    ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
        let result = reader.remaining_str();
        reader.to_end();
        ParsingResult {
            suggestions: None,
            result: Ok(Some(Self(result))),
        }
    }
}

impl<'a> BrigadierArgument<'a> for GreedyString<'a> {
    fn parser(_data: Option<&Self::Data>) -> Parser<'a> {
        Parser::String(StringArg::GreedyPhrase)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ArrayError<E> {
    Value(E),
    Separator,
}

impl<E: ParsingBuild<ParsingError>> ParsingBuild<ParsingError> for ArrayError<E> {
    fn build(self) -> ParsingError {
        match self {
            Self::Value(e) => e.build(),
            Self::Separator => ParsingError::translate(COMMAND_EXPECTED_SEPARATOR, vec![]),
        }
    }
}

impl<E> From<E> for ArrayError<E> {
    fn from(value: E) -> Self {
        Self::Value(value)
    }
}

impl<'a, const C: usize, T: Parse<'a>> Parse<'a> for [T; C]
where
    T::Data: Sized,
{
    type Data = [Option<T::Data>; C];

    type Error = ArrayError<T::Error>;

    type Suggestions = T::Suggestions;

    fn parse(
        data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        purpose: ParsingPurpose,
    ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
        // I hope that compiler will optimize this option thing.
        let mut values: Option<[MaybeUninit<T>; C]> = match purpose {
            // SAFETY: uninited MaybeUninit is not UB
            ParsingPurpose::Reading => Some(unsafe { MaybeUninit::uninit().assume_init() }),
            ParsingPurpose::Suggestion => None,
        };

        macro_rules! t_write {
            ($i:expr) => {{
                let t_result = T::parse(data.and_then(|v| v[$i].as_ref()), reader, purpose);
                let v = match t_result.result {
                    Ok(value) => value,
                    Err((pos, err)) => {
                        return ParsingResult {
                            suggestions: t_result.suggestions,
                            result: Err((pos, err.into())),
                        };
                    }
                };
                if let Some(ref mut values) = values {
                    values[$i].write(v.expect("Purpose is Reading, but the given value is none"));
                }
            }};
        }

        t_write!(0);

        for i in 1..C {
            let begin = reader.cursor();

            if reader.next_char() != Some(' ') {
                return ParsingResult {
                    suggestions: None,
                    result: Err((begin..reader.cursor(), ArrayError::Separator)),
                };
            }

            t_write!(i);
        }

        ParsingResult {
            suggestions: None,
            result: Ok(values.map(|values| unsafe { values.map(|v| v.assume_init()) })),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bool_test() {
        let mut reader = StrReader::new("true false bad");

        assert_eq!(
            bool::parse(None, &mut reader, ParsingPurpose::Reading),
            ParsingResult {
                suggestions: Some((0..4, BoolSuggestions)),
                result: Ok(Some(true))
            }
        );

        assert_eq!(reader.next_char(), Some(' '));

        assert_eq!(
            bool::parse(None, &mut reader, ParsingPurpose::Reading),
            ParsingResult {
                suggestions: Some((5..10, BoolSuggestions)),
                result: Ok(Some(false)),
            }
        );

        assert_eq!(reader.next_char(), Some(' '));

        assert_eq!(
            bool::parse(None, &mut reader, ParsingPurpose::Reading),
            ParsingResult {
                suggestions: Some((11..14, BoolSuggestions)),
                result: Err((11..14, BoolParsingError::Invalid("bad"))),
            }
        );
    }

    #[test]
    fn num_test() {
        let mut reader = StrReader::new("10 30 40.0 50.0");

        assert_eq!(
            i32::parse(None, &mut reader, ParsingPurpose::Reading),
            ParsingResult {
                suggestions: None,
                result: Ok(Some(10))
            }
        );

        assert_eq!(reader.next_char(), Some(' '));

        assert_eq!(
            i32::parse(
                Some(&(Some(40), None)),
                &mut reader,
                ParsingPurpose::Reading
            ),
            ParsingResult {
                suggestions: None,
                result: Err((3..5, I32Error::TooLow("30", 40)))
            }
        );

        unsafe { reader.set_cursor(5) };

        assert_eq!(reader.next_char(), Some(' '));

        assert_eq!(
            f32::parse(None, &mut reader, ParsingPurpose::Reading),
            ParsingResult {
                suggestions: None,
                result: Ok(Some(40.0))
            }
        );

        assert_eq!(reader.next_char(), Some(' '));

        assert_eq!(
            f32::parse(
                Some(&(None, Some(40.0))),
                &mut reader,
                ParsingPurpose::Reading
            ),
            ParsingResult {
                suggestions: None,
                result: Err((11..15, F32Error::TooBig("50.0", 40.0)))
            }
        );
    }

    #[test]
    fn string_test() {
        let mut reader = StrReader::new(r#"aba "aba aba" "aba"#);

        assert_eq!(
            SingleWordString::parse(None, &mut reader, ParsingPurpose::Reading),
            ParsingResult {
                suggestions: None,
                result: Ok(Some(SingleWordString("aba")))
            }
        );

        assert_eq!(reader.next_char(), Some(' '));

        assert_eq!(
            QuotableString::parse(None, &mut reader, ParsingPurpose::Reading),
            ParsingResult {
                suggestions: None,
                result: Ok(Some(QuotableString(Cow::Owned("aba aba".into()))))
            }
        );

        assert_eq!(reader.next_char(), Some(' '));

        assert_eq!(
            QuotableString::parse(None, &mut reader, ParsingPurpose::Reading),
            ParsingResult {
                suggestions: None,
                result: Err((14..18, UnclosedQuoteError)),
            }
        );
    }
}
