use std::borrow::Cow;
use std::ops::Range;

use valence_core::text::Text;

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
pub struct ParsingResult<'a, T: Parsable<'a>> {
    pub suggestions: Option<(Range<usize>, T::Suggestions)>,
    pub result: Result<Option<T>, (Range<usize>, T::Error)>,
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

    type Data: 'a;

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
    ) -> ParsingResult<'a, Self>;
}

impl<'a> ParsingBuild<ParsingSuggestions<'a>> for () {
    fn build(self) -> ParsingSuggestions<'a> {
        Cow::Borrowed(&[])
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
