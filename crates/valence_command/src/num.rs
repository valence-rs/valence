use valence_core::protocol::packet::command::Parser;
use valence_core::translation_key::{
    ARGUMENT_DOUBLE_BIG, ARGUMENT_DOUBLE_LOW, ARGUMENT_FLOAT_BIG, ARGUMENT_FLOAT_LOW,
    ARGUMENT_INTEGER_BIG, ARGUMENT_INTEGER_LOW, ARGUMENT_LONG_BIG, ARGUMENT_LONG_LOW,
    PARSING_BOOL_INVALID, PARSING_DOUBLE_INVALID, PARSING_FLOAT_INVALID, PARSING_INT_INVALID,
    PARSING_LONG_INVALID,
};

use crate::parse::{
    BrigadierArgument, Parse, ParseError, ParseResult, ParseSuggestions, Suggestion,
};
use crate::reader::{StrLocated, StrReader, StrSpan};

impl<'a> Parse<'a> for bool {
    type Data = ();

    type Query = ();

    type SuggestionsQuery = ();

    type Suggestions = ();

    fn parse(
        _data: &Self::Data,
        suggestions: &mut StrLocated<Self::Suggestions>,
        _query: &Self::Query,
        reader: &mut StrReader<'a>,
    ) -> ParseResult<Self> {
        reader.span_err_located(&mut suggestions.span, |reader| {
            match reader.read_unquoted_str() {
                "true" => Ok(true),
                "false" => Ok(false),
                o => Err(ParseError::translate(
                    PARSING_BOOL_INVALID,
                    vec![o.to_string().into()],
                )),
            }
        })
    }

    fn suggestions(_suggestions: &Self::Suggestions, _query: &Self::Query) -> ParseSuggestions<'a> {
        const SUGGESTIONS: &[Suggestion<'static>] =
            &[Suggestion::new_str("true"), Suggestion::new_str("false")];

        ParseSuggestions::Borrowed(SUGGESTIONS)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NumberParseError {
    Big,
    Low,
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumberBounds<T> {
    pub min: Option<T>,
    pub max: Option<T>,
}

impl<T> Default for NumberBounds<T> {
    fn default() -> Self {
        Self {
            min: None,
            max: None,
        }
    }
}

macro_rules! impl_num {
    (
        $ty:ident,
        $too_big:expr,
        $too_low:expr,
        $expected:expr,
        $parser:ident,
    ) => {

        paste::paste! {
            pub fn [<parse_ $ty>](bounds: &NumberBounds<$ty>, reader: &mut StrReader) -> Result<$ty, NumberParseError> {
                let num = reader.read_num_str().parse().map_err(|_| NumberParseError::Invalid)?;
                match bounds {
                    NumberBounds {
                        min: Some(min),
                        max: _
                    } if *min > num => Err(NumberParseError::Low),
                    NumberBounds {
                        min: _,
                        max: Some(max)
                    } if *max < num => Err(NumberParseError::Big),
                    _ => Ok(num),
                }
            }
        }

        impl<'a> Parse<'a> for $ty {
            type Data = NumberBounds<Self>;

            type Query = ();

            type SuggestionsQuery = ();

            type Suggestions = ();

            fn parse(
                data: &Self::Data,
                _suggestions: &mut StrLocated<Self::Suggestions>,
                _query: &Self::Query,
                reader: &mut StrReader<'a>,
            ) -> ParseResult<Self> {
                let begin = reader.cursor();
                let result = paste::paste! { [<parse_ $ty>](data, reader) };
                let span = StrSpan::new(begin, reader.cursor());

                // SAFETY: Span is created from valid cursors
                let str = unsafe { reader.get_str(span) };

                match result {
                    Ok(n) => Ok(n),
                    Err(NumberParseError::Big) => Err(ParseError::translate(
                        $too_big,
                        vec![
                            data.max.unwrap().to_string().into(),
                            str.to_string().into()
                        ]
                    )),
                    Err(NumberParseError::Low) => Err(ParseError::translate(
                        $too_low,
                        vec![
                            data.min.unwrap().to_string().into(),
                            str.to_string().into()
                        ]
                    )),
                    Err(NumberParseError::Invalid) => Err(ParseError::translate(
                        $expected,
                        vec![str.to_string().into()]
                    ))
                }.map_err(|err| StrLocated::new(span, err))
            }

        }

        impl<'a> BrigadierArgument<'a> for $ty {
            fn parser(data: NumberBounds<Self>) -> Parser<'a> {
                Parser::$parser {
                    min: data.min,
                    max: data.max,
                }
            }
        }
    };
}

impl_num!(
    i32,
    ARGUMENT_INTEGER_BIG,
    ARGUMENT_INTEGER_LOW,
    PARSING_INT_INVALID,
    Integer,
);

impl_num!(
    i64,
    ARGUMENT_LONG_BIG,
    ARGUMENT_LONG_LOW,
    PARSING_LONG_INVALID,
    Long,
);

impl_num!(
    f32,
    ARGUMENT_FLOAT_BIG,
    ARGUMENT_FLOAT_LOW,
    PARSING_FLOAT_INVALID,
    Float,
);

impl_num!(
    f64,
    ARGUMENT_DOUBLE_BIG,
    ARGUMENT_DOUBLE_LOW,
    PARSING_DOUBLE_INVALID,
    Double,
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn num_tests() {
        assert_eq!(
            i32::parse(
                &NumberBounds::default(),
                &mut Default::default(),
                &(),
                &mut StrReader::new("64")
            ),
            Ok(64),
        )
    }
}
