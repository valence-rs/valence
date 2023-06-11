use valence_core::protocol::packet::command::Parser;
use valence_core::translation_key::{
    ARGUMENT_RANGE_EMPTY, ARGUMENT_RANGE_INTS, ARGUMENT_RANGE_SWAPPED,
};

use crate::p_try;
use crate::parser::{
    BrigadierArgument, NoParsingBuild, Parse, ParsingBuild, ParsingError, ParsingPurpose,
    ParsingResult,
};
use crate::reader::StrReader;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct InclusiveRange<T> {
    pub min: Option<T>,
    pub max: Option<T>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InclusiveRangeError<E> {
    Empty,
    Ints,
    Swapped,
    Num(E),
}

impl<E: ParsingBuild<ParsingError>> ParsingBuild<ParsingError> for InclusiveRangeError<E> {
    fn build(self) -> ParsingError {
        match self {
            Self::Empty => ParsingError::translate(ARGUMENT_RANGE_EMPTY, vec![]),
            Self::Ints => ParsingError::translate(ARGUMENT_RANGE_INTS, vec![]),
            Self::Swapped => ParsingError::translate(ARGUMENT_RANGE_SWAPPED, vec![]),
            Self::Num(e) => e.build(),
        }
    }
}

impl<E> From<E> for InclusiveRangeError<E> {
    fn from(value: E) -> Self {
        Self::Num(value)
    }
}

macro_rules! inclusive_range_impl {
    ($($ty:ty, $int: expr,)*) => {
        $(impl<'a> Parse<'a> for InclusiveRange<$ty> {
            type Data = <$ty as Parse<'a>>::Data;

            type Suggestions = NoParsingBuild;

            type Error = InclusiveRangeError<<$ty as Parse<'a>>::Error>;

            fn parse(
                data: Option<&Self::Data>,
                reader: &mut StrReader<'a>,
                purpose: ParsingPurpose,
            ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
                macro_rules! read {
                    () => {
                        p_try!(<$ty>::parse(data, reader, purpose)).1
                    };
                }

                let begin = reader.cursor();

                if reader.remaining_str().starts_with("..") {
                    reader.skip_next_chars(2);
                    let max = read!();
                    ParsingResult {
                        suggestions: None,
                        result: Ok(max.map(|max| Self { min: None, max: Some(max) })),
                    }
                } else {
                    let min = read!();
                    if reader.remaining_str().starts_with("..") {
                        reader.skip_next_chars(2);
                        if matches!(reader.peek_char(), Some('0'..='9') | Some('+') | Some('-')) {
                            let max = read!();
                            ParsingResult {
                                suggestions: None,
                                result: match (min, max) {
                                    (Some(min), Some(max)) if min > max => {
                                        Err((begin..reader.cursor(), InclusiveRangeError::Swapped))
                                    }
                                    (Some(min), Some(max)) => Ok(Some(Self { min: Some(min), max: Some(max) })),
                                    (..) => Ok(None),
                                },
                            }
                        } else {
                            ParsingResult {
                                suggestions: None,
                                result: Ok(min.map(|min| Self {
                                    min: Some(min),
                                    max: None,
                                }))
                            }
                        }
                    } else {
                        ParsingResult {
                            suggestions: None,
                            result: Ok(min.map(|val| Self { min: Some(val), max: Some(val) })),
                        }
                    }
                }
            }
        }

        impl<'a> BrigadierArgument<'a> for InclusiveRange<$ty> {
            fn parser(_data: Option<&Self::Data>) -> Parser<'a> {
                if $int {
                    Parser::IntRange
                } else {
                    Parser::FloatRange
                }
            }
        })*
    };
}

inclusive_range_impl!(i32, true, i64, true, f32, false, f64, false,);

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn range_test() {
        assert_eq!(
            InclusiveRange::parse(None, &mut StrReader::new("0..10"), ParsingPurpose::Reading,),
            ParsingResult {
                suggestions: None,
                result: Ok(Some(InclusiveRange {
                    min: Some(0),
                    max: Some(10)
                }))
            }
        );

        assert_eq!(
            InclusiveRange::parse(None, &mut StrReader::new("..10"), ParsingPurpose::Reading,),
            ParsingResult {
                suggestions: None,
                result: Ok(Some(InclusiveRange {
                    min: None,
                    max: Some(10)
                }))
            }
        );

        assert_eq!(
            InclusiveRange::parse(None, &mut StrReader::new("10"), ParsingPurpose::Reading,),
            ParsingResult {
                suggestions: None,
                result: Ok(Some(InclusiveRange {
                    min: Some(10),
                    max: Some(10)
                }))
            }
        );

        assert_eq!(
            InclusiveRange::parse(None, &mut StrReader::new("10.."), ParsingPurpose::Reading,),
            ParsingResult {
                suggestions: None,
                result: Ok(Some(InclusiveRange {
                    min: Some(10.0),
                    max: None
                }))
            }
        );
    }
}
