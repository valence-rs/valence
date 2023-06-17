use std::borrow::Cow;

use bevy_ecs::system::ReadOnlySystemParam;
use valence_core::protocol::packet::command::Parser;
use valence_core::text::Text;

use crate::reader::{StrLocated, StrReader, StrSpan};

#[derive(Clone, Debug, PartialEq)]
pub struct Suggestion<'a> {
    pub value: Cow<'a, str>,
    pub tooltip: Option<Text>,
}

impl<'a> Suggestion<'a> {
    pub const fn new_str(str: &'a str) -> Self {
        Self {
            value: Cow::Borrowed(str),
            tooltip: None,
        }
    }

    pub const fn new(value: Cow<'a, str>) -> Self {
        Self {
            value,
            tooltip: None,
        }
    }
}

impl<'a> From<String> for Suggestion<'a> {
    fn from(value: String) -> Self {
        Suggestion::new(Cow::Owned(value))
    }
}

pub type ParseError = Text;
pub type ParseResult<T> = Result<T, StrLocated<ParseError>>;
pub type ParseSuggestions<'a> = Cow<'a, [Suggestion<'a>]>;

pub trait Parse<'a>: Sized + 'a {
    type Data;

    type Query: ReadOnlySystemParam;

    type Suggestions: Default + 'a;

    fn parse(
        data: &Self::Data,
        _suggestions: &mut Self::Suggestions,
        _query: &Self::Query,
        reader: &mut StrReader<'a>,
    ) -> ParseResult<Self>;

    /// Do the same as [`Parse::parse`] but does not return parse object
    fn skip(
        data: &Self::Data,
        _suggestions: &mut Self::Suggestions,
        _query: &Self::Query,
        reader: &mut StrReader<'a>,
    ) -> ParseResult<()> {
        Self::parse(data, _suggestions, _query, reader).map(|_| ())
    }

    /// Returns the list of suggestions for given suggestions.
    /// If the suggestions list is empty than it doesn't matter which span is
    /// given.
    ///
    /// ### Default
    /// Returns empty suggestion list
    fn suggestions(
        _data: &Self::Data,
        _result: &ParseResult<()>,
        _suggestions: &Self::Suggestions,
        _query: &Self::Query,
    ) -> StrLocated<ParseSuggestions<'a>> {
        no_suggestions()
    }
}

pub const fn no_suggestions<'a>() -> StrLocated<ParseSuggestions<'a>> {
    StrLocated::new(StrSpan::start(), ParseSuggestions::Borrowed(&[]))
}

pub trait BrigadierArgument<'a>: Parse<'a> {
    fn parser(data: Self::Data) -> Parser<'a>;
}

#[cfg(test)]
pub fn parse_test<'a, T: Parse<'a>>(
    data: T::Data,
    suggestions: &mut T::Suggestions,
    query: &T::Query,
    reader: &mut StrReader<'a>,
    chars_read: usize,
    expected: ParseResult<T>,
) where
    T: PartialEq,
    T: std::fmt::Debug,
    T::Suggestions: PartialEq,
    T::Suggestions: std::fmt::Debug,
{
    let mut skip_reader = reader.clone();

    let result = T::parse(&data, suggestions, query, reader);
    assert_eq!(result, expected);
    assert_eq!(
        reader.cursor().chars() - skip_reader.cursor().chars(),
        chars_read
    );

    let mut skip_suggestions = T::Suggestions::default();
    let skip_result = T::skip(&data, &mut skip_suggestions, query, &mut skip_reader);

    assert_eq!(result.map(|_| ()), skip_result);
    assert_eq!(skip_reader.cursor(), reader.cursor());
    assert_eq!(suggestions, &mut skip_suggestions);
}
