use std::ops::Add;

use valence_core::translation_key::ARGUMENT_POS_MIXED;

use crate::parse::{Parse, ParseError, ParseResult};
use crate::reader::{StrLocated, StrReader, StrSpan};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WorldPosition<T> {
    Relative(T),
    Absolute(T),
}

impl<T: Add<Output = T>> WorldPosition<T> {
    pub fn apply(self, value: T) -> T {
        match self {
            Self::Relative(v) => v + value,
            Self::Absolute(v) => v,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct WorldPositionData<D> {
    pub absolute: D,
    pub relative: D,
}

impl<'a, T: Parse<'a>> Parse<'a> for WorldPosition<T> {
    type Data = WorldPositionData<T::Data>;

    type Query = T::Query;

    type SuggestionsQuery = ();

    type Suggestions = ();

    fn parse(
        data: &Self::Data,
        suggestions: &mut StrLocated<Self::Suggestions>,
        query: &Self::Query,
        reader: &mut StrReader<'a>,
    ) -> ParseResult<Self> {
        reader.span_located(&mut suggestions.span, |reader| {
            let begin = reader.cursor();

            let result = match reader.peek_char() {
                Some('^') => {
                    reader.next_char();
                    Err(ParseError::translate(ARGUMENT_POS_MIXED, vec![]))
                }
                Some('~') => {
                    reader.next_char();
                    Ok(true)
                }
                _ => Ok(false),
            };

            match result {
                Ok(relative) => {
                    let value = T::parse(
                        if relative {
                            &data.relative
                        } else {
                            &data.absolute
                        },
                        &mut Default::default(),
                        query,
                        reader,
                    )?;

                    Ok(if relative {
                        Self::Relative(value)
                    } else {
                        Self::Absolute(value)
                    })
                }
                Err(err) => Err(StrLocated::new(StrSpan::new(begin, reader.cursor()), err)),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_test;

    #[test]
    fn world_position_test() {
        parse_test(
            &Default::default(),
            &mut Default::default(),
            &(),
            &mut StrReader::new("~32 ~64 super"),
            3,
            Ok(WorldPosition::Relative(32i32)),
        );

        parse_test(
            &Default::default(),
            &mut Default::default(),
            &(),
            &mut StrReader::new("32 ~64 super"),
            2,
            Ok(WorldPosition::Absolute(32i32)),
        );
    }
}
