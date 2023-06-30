use std::borrow::Cow;

use valence_core::text::Text;
use valence_core::translation_key::PARSING_BOOL_INVALID;

use crate::parse::{CommandExecutor, Parse, ParseResult, ParseSuggestions, Suggestion};
use crate::reader::{StrLocated, StrReader, StrSpan};
use crate::suggestions_impl;

impl<'a> Parse<'a> for bool {
    type Data = ();

    type Query = ();

    type Suggestions = StrSpan;

    fn parse(
        _data: &Self::Data,
        suggestions: &mut Self::Suggestions,
        _query: &Self::Query,
        reader: &mut StrReader<'a>,
    ) -> ParseResult<Self> {
        reader.span_err_located(suggestions, |reader| {
            let str = reader.read_unquoted_str();
            if str.eq_ignore_ascii_case("true") {
                Ok(true)
            } else if str.eq_ignore_ascii_case("false") {
                Ok(false)
            } else {
                Err(Text::translate(
                    PARSING_BOOL_INVALID,
                    vec![str.to_string().into()],
                ))
            }
        })
    }
}

impl<'a> ParseSuggestions<'a> for bool {
    type SuggestionsQuery = ();

    fn suggestions(
        _executor: CommandExecutor,
        _query: &Self::SuggestionsQuery,
        str: String,
        suggestions: Self::Suggestions,
    ) -> StrLocated<Cow<'a, [Suggestion<'a>]>> {
        const EMPTY: &[Suggestion<'static>] = &[];
        const ONLY_TRUE: &[Suggestion<'static>] = &[Suggestion::new_str("true")];
        const ONLY_FALSE: &[Suggestion<'static>] = &[Suggestion::new_str("false")];
        const BOTH: &[Suggestion<'static>] =
            &[Suggestion::new_str("true"), Suggestion::new_str("false")];

        let str = suggestions
            .in_str(&str)
            .expect("Given string is not the one parse has used");

        if str.len() > 5 {
            return StrLocated::new(suggestions, Cow::Borrowed(EMPTY));
        }

        let lc_str = str.to_ascii_lowercase();

        let result = if str.len() == 0 {
            BOTH
        } else if "true".starts_with(&lc_str) {
            ONLY_TRUE
        } else if "false".starts_with(&lc_str) {
            ONLY_FALSE
        } else {
            EMPTY
        };

        StrLocated::new(suggestions, Cow::Borrowed(result))
    }
}

suggestions_impl!(bool);
