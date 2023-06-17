use std::marker::PhantomData;
use std::mem::MaybeUninit;

use valence_core::translation_key::PARSING_EXPECTED;

use crate::parse::{no_suggestions, Parse, ParseError, ParseResult, ParseSuggestions};
use crate::reader::{StrLocated, StrReader};

#[derive(Clone, Copy, Debug)]
pub struct ArrayData<D, const L: usize>(pub [D; L]);

impl<D: Default, const L: usize> Default for ArrayData<D, L> {
    fn default() -> Self {
        // SAFETY: uninited array of MUs is not UB
        let mut result: [MaybeUninit<D>; L] = unsafe { MaybeUninit::uninit().assume_init() };

        for mu in &mut result {
            mu.write(D::default());
        }

        // SAFETY: each value of result is initialized
        Self(result.map(|v| unsafe { v.assume_init() }))
    }
}

impl<D, const L: usize> From<[D; L]> for ArrayData<D, L> {
    fn from(value: [D; L]) -> Self {
        Self(value)
    }
}

/// Parses L elements delimitted with whitespace
/// ### Example
/// 32 64 96 = [32, 64, 96] if T is a number
impl<'a, T: Parse<'a>, const L: usize> Parse<'a> for [T; L] {
    type Data = ArrayData<T::Data, L>;

    type Query = T::Query;

    type Suggestions = (usize, T::Suggestions);

    fn parse(
        data: &Self::Data,
        suggestions: &mut Self::Suggestions,
        query: &Self::Query,
        reader: &mut StrReader<'a>,
    ) -> ParseResult<Self> {
        // SAFETY: uninited array of MUs is not UB
        let mut result: [MaybeUninit<T>; L] = unsafe { MaybeUninit::uninit().assume_init() };

        // Data is the same length as result (L) so we will write something to each
        // value of result
        for (i, data) in data.0.iter().enumerate() {
            suggestions.0 = i;
            result[i].write(T::parse(data, &mut suggestions.1, query, reader)?);
            reader.err_located(|reader| {
                if i != L - 1 && !reader.skip_char(' ') {
                    Err(ParseError::translate(PARSING_EXPECTED, vec![" ".into()]))
                } else {
                    Ok(())
                }
            })?;
        }

        // SAFETY: each value of result is initialized
        Ok(result.map(|v| unsafe { v.assume_init() }))
    }

    fn skip(
        data: &Self::Data,
        suggestions: &mut Self::Suggestions,
        query: &Self::Query,
        reader: &mut StrReader<'a>,
    ) -> ParseResult<()> {
        for (i, data) in data.0.iter().enumerate() {
            suggestions.0 = i;
            T::skip(data, &mut suggestions.1, query, reader)?;
            reader.err_located(|reader| {
                if i != L - 1 && !reader.skip_char(' ') {
                    Err(ParseError::translate(PARSING_EXPECTED, vec![" ".into()]))
                } else {
                    Ok(())
                }
            })?;
        }

        Ok(())
    }

    fn suggestions(
        data: &Self::Data,
        result: &ParseResult<()>,
        suggestions: &Self::Suggestions,
        query: &Self::Query,
    ) -> StrLocated<ParseSuggestions<'a>> {
        T::suggestions(&data.0[suggestions.0], result, &suggestions.1, query)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DynArray<T>(PhantomData<T>);

pub struct DynArrayData<D> {
    pub end: char,
    pub delim: Option<char>,
    pub delim_err: ParseError,
    pub inner_data: D,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum DynArraySuggestions<S> {
    #[default]
    None,
    Delim,
    Inherit(S),
}

impl<'a, T: Parse<'a>> DynArray<T> {
    fn callback_skip(
        data: &DynArrayData<T::Data>,
        suggestions: &mut DynArraySuggestions<T::Suggestions>,
        query: &T::Query,
        reader: &mut StrReader<'a>,
        mut callback: impl FnMut(
            &T::Data,
            &mut T::Suggestions,
            &T::Query,
            &mut StrReader<'a>,
        ) -> ParseResult<()>,
    ) -> ParseResult<()> {
        let DynArrayData {
            end,
            delim,
            delim_err,
            inner_data,
        } = data;

        if reader.skip_char(*end) {
            Ok(())
        } else {
            loop {
                let mut value_suggestions = T::Suggestions::default();
                if let Err(err) = callback(inner_data, &mut value_suggestions, query, reader) {
                    *suggestions = DynArraySuggestions::Inherit(value_suggestions);
                    break Err(err);
                }

                reader.skip_char(' ');

                match reader.err_located(|reader| {
                    let ch = reader.next_char();
                    if ch == Some(*end) {
                        Ok(true)
                    } else if delim.is_some() && ch.eq(delim) || delim.is_none() {
                        Ok(false)
                    } else {
                        Err(delim_err.clone())
                    }
                }) {
                    Ok(true) => {
                        break Ok(());
                    }
                    Ok(false) => {}
                    Err(err) => {
                        *suggestions = DynArraySuggestions::Delim;
                        return Err(err);
                    }
                }

                reader.skip_char(' ');
            }
        }?;
        *suggestions = DynArraySuggestions::None;
        Ok(())
    }

    pub fn parse(
        data: &DynArrayData<T::Data>,
        suggestions: &mut DynArraySuggestions<T::Suggestions>,
        query: &T::Query,
        reader: &mut StrReader<'a>,
        mut callback: impl FnMut(T),
    ) -> ParseResult<()> {
        Self::callback_skip(
            data,
            suggestions,
            query,
            reader,
            |data, suggestions, query, reader| {
                callback(T::parse(data, suggestions, query, reader)?);
                Ok(())
            },
        )
    }

    pub fn skip(
        data: &DynArrayData<T::Data>,
        suggestions: &mut DynArraySuggestions<T::Suggestions>,
        query: &T::Query,
        reader: &mut StrReader<'a>,
    ) -> ParseResult<()> {
        Self::callback_skip(
            data,
            suggestions,
            query,
            reader,
            |data, suggestions, query, reader| T::skip(data, suggestions, query, reader),
        )
    }

    pub fn suggestions(
        data: &DynArrayData<T::Data>,
        result: &ParseResult<()>,
        suggestions: &mut DynArraySuggestions<T::Suggestions>,
        query: &T::Query,
    ) -> StrLocated<ParseSuggestions<'a>> {
        match (suggestions, result) {
            (DynArraySuggestions::None, _) => no_suggestions(),
            (DynArraySuggestions::Delim, Err(StrLocated { span, .. })) => StrLocated::new(
                *span,
                // TODO: Really don't like heap allocation
                ParseSuggestions::Owned(match data.delim {
                    Some(delim) => vec![delim.to_string().into(), data.end.to_string().into()],
                    None => vec![data.end.to_string().into()],
                }),
            ),
            (DynArraySuggestions::Delim, Ok(_)) => {
                unreachable!("Delim suggestions are only available when error occured")
            }
            (DynArraySuggestions::Inherit(inherit), result) => {
                T::suggestions(&data.inner_data, result, inherit, query)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use valence_core::text::Text;

    use super::*;
    use crate::parse::parse_test;

    #[test]
    fn const_arr_test() {
        parse_test(
            ArrayData::default(),
            &mut Default::default(),
            &(),
            &mut StrReader::new("32 64 96 128"),
            2 * 3 + 2,
            Ok([32, 64, 96]),
        );
    }

    #[test]
    fn dyn_arr_test() {
        let mut iter = [32, 64, 96, 128, 256].into_iter();
        DynArray::parse(
            &DynArrayData {
                delim: Some(','),
                delim_err: Text::text("delim required"),
                end: ']',
                inner_data: Default::default(),
            },
            &mut Default::default(),
            &(),
            &mut StrReader::new("32, 64, 96, 128 , 256]"),
            |value| {
                assert_eq!(Some(value), iter.next());
            },
        ).unwrap();
        assert_eq!(iter.next(), None);
    }
}
