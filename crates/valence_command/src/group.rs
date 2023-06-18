use std::marker::PhantomData;

use valence_core::translation_key::PARSING_EXPECTED;

use crate::parse::{Parse, ParseError, ParseResult};
use crate::reader::{StrLocated, StrReader};

pub struct ParseGroup<'b, 'a, S, C> {
    pub reader: &'b mut StrReader<'a>,
    pub values: C,
    _marker: PhantomData<S>,
}

impl<'b, 'a, S> ParseGroup<'b, 'a, S, ()> {
    pub fn new(reader: &'b mut StrReader<'a>) -> Self {
        Self {
            reader,
            values: (),
            _marker: PhantomData,
        }
    }
}

impl<'b, 'a, S, C> ParseGroup<'b, 'a, S, C> {
    pub fn next<T: Parse<'a>>(
        self,
        data: &T::Data,
        suggestions: &mut StrLocated<S>,
        query: &T::Query,
    ) -> ParseResult<ParseGroup<'b, 'a, S, (C, T)>>
    where
        S: From<T::Suggestions>,
    {
        let mut t_suggestions = Default::default();
        let t = T::parse(data, &mut t_suggestions, query, self.reader)?;

        *suggestions = t_suggestions.map(|v| v.into());

        Ok(ParseGroup {
            reader: self.reader,
            values: (self.values, t),
            _marker: PhantomData,
        })
    }

    pub fn skip<T: Parse<'a>>(
        self,
        data: &T::Data,
        suggestions: &mut StrLocated<S>,
        query: &T::Query,
    ) -> ParseResult<Self>
    where
        S: From<T::Suggestions>,
    {
        let mut t_suggestions = Default::default();
        T::skip(data, &mut t_suggestions, query, self.reader)?;

        *suggestions = t_suggestions.map(|v| v.into());

        Ok(self)
    }

    pub fn token(self, token: char, suggestions: &mut StrLocated<S>) -> ParseResult<Self>
    where
        S: From<char>,
    {
        match self
            .reader
            .span_err_located(&mut suggestions.span, |reader| {
                if reader.skip_char(token) {
                    Ok(())
                } else {
                    Err(ParseError::translate(
                        PARSING_EXPECTED,
                        vec![token.to_string().into()],
                    ))
                }
            }) {
            Ok(_) => Ok(self),
            Err(e) => {
                suggestions.object = token.into();
                Err(e)
            }
        }
    }

    pub fn optional_token(self, token: char) -> Self {
        self.reader.skip_char(token);
        self
    }
}
