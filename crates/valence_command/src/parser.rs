use std::borrow::Cow;
use std::ops::Range;

use valence_core::protocol::packet::command::Parser;
use valence_core::text::Text;

use crate::parsing_ret_err;
use crate::reader::StrReader;

#[derive(Clone, Debug)]
pub struct Suggestion<'a> {
    pub message: Cow<'a, str>,
    pub tooltip: Option<Text>,
}

impl<'a> Suggestion<'a> {
    pub const fn new_str(message: &'a str) -> Self {
        Self {
            message: Cow::Borrowed(message),
            tooltip: None,
        }
    }
}

impl<'a> From<&'a str> for Suggestion<'a> {
    fn from(value: &'a str) -> Self {
        Self {
            message: Cow::Borrowed(value),
            tooltip: None,
        }
    }
}

impl<'a> From<String> for Suggestion<'a> {
    fn from(value: String) -> Self {
        Self {
            message: Cow::Owned(value),
            tooltip: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParsingResult<T, S, E> {
    pub suggestions: Option<(Range<usize>, S)>,
    pub result: Result<Option<T>, (Range<usize>, E)>,
}

impl<T, S, E> ParsingResult<T, S, E> {
    pub const fn ok() -> Self {
        Self {
            suggestions: None,
            result: Ok(None),
        }
    }

    pub fn map_suggestion<S1>(self, func: impl FnOnce(S) -> S1) -> ParsingResult<T, S1, E> {
        ParsingResult {
            suggestions: self.suggestions.map(|(pos, s)| (pos, func(s))),
            result: self.result,
        }
    }

    pub fn map_ok<T1>(self, func: impl FnOnce(T) -> T1) -> ParsingResult<T1, S, E> {
        ParsingResult {
            suggestions: self.suggestions,
            result: self.result.map(|v| v.map(func)),
        }
    }

    pub fn map_err<E1>(self, func: impl FnOnce(E) -> E1) -> ParsingResult<T, S, E1> {
        ParsingResult {
            suggestions: self.suggestions,
            result: self.result.map_err(|(pos, e)| (pos, func(e))),
        }
    }

    pub fn zip<T1>(
        self,
        func: impl FnOnce() -> ParsingResult<T1, S, E>,
    ) -> ParsingResult<(T, T1), S, E> {
        let res = parsing_ret_err!(self);

        let other = func();

        ParsingResult {
            suggestions: other.suggestions,
            // parsing_ret_err! ensures that result is Ok
            result: other.result.map(|v| match (v, res.result) {
                (Some(v), Ok(Some(o))) => Some((o, v)),
                _ => None,
            }),
        }
    }
}

/// Macro ensures that returned result contains no Error
#[macro_export]
macro_rules! parsing_ret_err {
    ($res:expr) => {{
        let res = $res;
        if let Err(err) = res.result {
            return $crate::parser::ParsingResult {
                suggestions: res.suggestions,
                result: Err(err),
            };
        }
        res
    }};
}

pub trait ParsingBuild<T> {
    fn build(self) -> T;
}

pub type ParsingError = Text;
pub type ParsingSuggestions<'a> = Cow<'a, [Suggestion<'a>]>;

#[derive(Clone, Copy, Debug)]
pub enum ParsingPurpose {
    Suggestion,
    Reading,
}

#[macro_export]
macro_rules! parsing_error {
    ($name: ident {
        $($error_name: ident = $key: expr$(,)?)*
    }) => {
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum $name {
            $($error_name,)*
        }

        impl $crate::parser::ParsingBuild<$crate::parser::ParsingError> for $name {
            fn build(self) -> $crate::parser::ParsingError {
                match self {
                    $(Self::$error_name => valence_core::text::Text::translate($key, vec![]),)*
                }
            }
        }
    };
    ($name: ident = $key: expr) => {
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub struct $name;

        impl $crate::parser::ParsingBuild<$crate::parser::ParsingError> for $name {
            fn build(self) -> $crate::parser::ParsingError {
                valence_core::text::Text::translate($key, vec![])
            }
        }
    }
}

pub trait Parsable<'a>: 'a + Sized {
    type Error: 'a + ParsingBuild<ParsingError> + Sized;

    type Suggestions: 'a + ParsingBuild<ParsingSuggestions<'a>> + Sized;

    type Data: 'a + ?Sized;

    /// The result can depend on `purpose` value:
    /// - [`ParsingPurpose::Suggestion`]
    ///     - The object **may** be [None]
    ///     - The error must be given if any found
    ///     - The suggestions must be given if any found
    /// - [`ParsingPurpose::Reading`]
    ///     - The object must be given if no errors occured
    ///     - The error must be given if any found
    ///     - The suggestions **may** be [None] in any situation
    ///
    /// Parsing on the same string value, the same data and the same purpose
    /// must give the same [ParsingResult]
    fn parse(
        data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        purpose: ParsingPurpose,
    ) -> ParsingResult<Self, Self::Suggestions, Self::Error>;
}

impl<'a> ParsingBuild<ParsingSuggestions<'a>> for () {
    fn build(self) -> ParsingSuggestions<'a> {
        Cow::Borrowed(&[])
    }
}

impl ParsingBuild<ParsingError> for () {
    fn build(self) -> ParsingError {
        ParsingError::text("error")
    }
}

// TODO change to never type (!), when it stabilizes

/// Indicates any [ParsingBuild] that can not occur.
/// Examples:
/// - The object with no suggestions
/// - The object which is parsed in any situation
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NoParsingBuild {}

impl<T> ParsingBuild<T> for NoParsingBuild {
    fn build(self) -> T {
        unreachable!()
    }
}

#[macro_export]
macro_rules! parsing_token {
    ($reader:expr, $token:expr, $error:expr, $suggestions:expr $(,)?) => {{
        let begin = $reader.cursor();
        if !$reader.skip_char($token) {
            return $crate::parser::ParsingResult {
                suggestions: Some((begin..$reader.cursor(), $suggestions)),
                result: Err((begin..$reader.cursor(), $error)),
            };
        }
    }};
}

pub trait BrigadierArgument<'a>: Parsable<'a> {
    fn parser(data: Option<&Self::Data>) -> Parser<'a>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsing_ret_err;

    #[test]
    fn ret_err() {
        fn func() -> ParsingResult<(), (), i32> {
            parsing_ret_err!(ParsingResult::<(), (), _> {
                suggestions: None,
                result: Err((0..0, 0))
            });
            unreachable!()
        }
        assert!(func().result.is_err());
    }
}
