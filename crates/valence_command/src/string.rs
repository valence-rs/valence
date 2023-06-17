use std::borrow::Cow;

use valence_core::protocol::packet::command::{Parser, StringArg};
use valence_core::text::Text;
use valence_core::translation_key::PARSING_QUOTE_EXPECTED_END;

use crate::parse::{BrigadierArgument, Parse, ParseResult};
use crate::reader::StrReader;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SingleWordString<'a>(pub &'a str);

impl<'a> Parse<'a> for SingleWordString<'a> {
    type Data = ();

    type Query = ();

    type Suggestions = ();

    fn parse(
        _data: Self::Data,
        _suggestions: &mut Self::Suggestions,
        _query: &Self::Query,
        reader: &mut StrReader<'a>,
    ) -> ParseResult<Self> {
        Ok(Self(reader.read_unquoted_str()))
    }
}

impl<'a> BrigadierArgument<'a> for SingleWordString<'a> {
    fn parser(_data: Self::Data) -> Parser<'a> {
        Parser::String(StringArg::SingleWord)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct QuotablePhraseString<'a>(pub Cow<'a, str>);

impl<'a> Parse<'a> for QuotablePhraseString<'a> {
    type Data = ();

    type Query = ();

    type Suggestions = ();

    fn parse(
        _data: Self::Data,
        _suggestions: &mut Self::Suggestions,
        _query: &Self::Query,
        reader: &mut StrReader<'a>,
    ) -> ParseResult<Self> {
        reader
            .err_located(|reader| {
                if reader.skip('"') {
                    match reader.read_started_quoted_str() {
                        Some(str) => Ok(Cow::Owned(str)),
                        None => Err(Text::translate(PARSING_QUOTE_EXPECTED_END, vec![])),
                    }
                } else {
                    Ok(Cow::Borrowed(reader.read_unquoted_str()))
                }
            })
            .map(Self)
    }

    fn skip(
        _data: Self::Data,
        _suggestions: &mut Self::Suggestions,
        _query: &Self::Query,
        reader: &mut StrReader<'a>,
    ) -> ParseResult<()> {
        reader.err_located(|reader| {
            if reader.skip('"') {
                match reader.skip_started_quoted_str() {
                    true => Ok(()),
                    false => Err(Text::translate(PARSING_QUOTE_EXPECTED_END, vec![])),
                }
            } else {
                reader.read_unquoted_str();
                Ok(())
            }
        })
    }
}

impl<'a> BrigadierArgument<'a> for QuotablePhraseString<'a> {
    fn parser(_data: Self::Data) -> Parser<'a> {
        Parser::String(StringArg::QuotablePhrase)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GreedyString<'a>(pub &'a str);

impl<'a> Parse<'a> for GreedyString<'a> {
    type Data = ();

    type Query = ();

    type Suggestions = ();

    fn parse(
        _data: Self::Data,
        _suggestions: &mut Self::Suggestions,
        _query: &Self::Query,
        reader: &mut StrReader<'a>,
    ) -> ParseResult<Self> {
        let res = reader.remaining_str();
        reader.to_end();
        Ok(Self(res))
    }
}

impl<'a> BrigadierArgument<'a> for GreedyString<'a> {
    fn parser(_data: Self::Data) -> Parser<'a> {
        Parser::String(StringArg::GreedyPhrase)
    }
}

impl<'a> Parse<'a> for Cow<'a, str> {
    type Data = StringArg;

    type Query = ();

    type Suggestions = ();

    fn parse(
        data: Self::Data,
        _suggestions: &mut Self::Suggestions,
        _query: &Self::Query,
        reader: &mut StrReader<'a>,
    ) -> ParseResult<Self> {
        match data {
            StringArg::SingleWord => {
                SingleWordString::parse((), &mut (), &(), reader).map(|v| Cow::Borrowed(v.0))
            }
            StringArg::GreedyPhrase => {
                GreedyString::parse((), &mut (), &(), reader).map(|v| Cow::Borrowed(v.0))
            }
            StringArg::QuotablePhrase => {
                QuotablePhraseString::parse((), &mut (), &(), reader).map(|v| v.0)
            }
        }
    }

    fn skip(
        data: Self::Data,
        _suggestions: &mut Self::Suggestions,
        _query: &Self::Query,
        reader: &mut StrReader<'a>,
    ) -> ParseResult<()> {
        match data {
            StringArg::SingleWord => SingleWordString::skip((), &mut (), &(), reader),
            StringArg::GreedyPhrase => GreedyString::skip((), &mut (), &(), reader),
            StringArg::QuotablePhrase => QuotablePhraseString::skip((), &mut (), &(), reader),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IdentRef<'a> {
    pub namespace: Option<&'a str>,
    pub key: &'a str,
}

impl<'a> Parse<'a> for IdentRef<'a> {
    type Data = ();

    type Query = ();

    type Suggestions = ();

    fn parse(
        _data: Self::Data,
        _suggestions: &mut Self::Suggestions,
        _query: &Self::Query,
        reader: &mut StrReader<'a>,
    ) -> ParseResult<Self> {
        let (namespace, key) = reader.read_ident_str();
        Ok(Self { namespace, key })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_test;

    #[test]
    fn single_word_test() {
        parse_test(
            (),
            &mut (),
            &(),
            &mut StrReader::new("hello world"),
            5,
            Ok(SingleWordString("hello")),
        );

        parse_test(
            StringArg::SingleWord,
            &mut (),
            &(),
            &mut StrReader::new("bye world"),
            3,
            Ok(Cow::Borrowed("bye")),
        );
    }

    #[test]
    fn quotable_phrase_test() {
        parse_test(
            (),
            &mut (),
            &(),
            &mut StrReader::new("without quotes"),
            7,
            Ok(QuotablePhraseString(Cow::Borrowed("without"))),
        );

        parse_test(
            (),
            &mut (),
            &(),
            &mut StrReader::new(r#""with quotes""#),
            13,
            Ok(QuotablePhraseString(Cow::Owned("with quotes".into()))),
        );

        parse_test(
            StringArg::QuotablePhrase,
            &mut (),
            &(),
            &mut StrReader::new("hello world"),
            5,
            Ok(Cow::Borrowed("hello")),
        );

        parse_test(
            StringArg::QuotablePhrase,
            &mut (),
            &(),
            &mut StrReader::new(r#""hello world""#),
            13,
            Ok(Cow::Owned("hello world".into())),
        );
    }

    #[test]
    fn greedy_test() {
        parse_test(
            (),
            &mut (),
            &(),
            &mut StrReader::new("1 2 3"),
            5,
            Ok(GreedyString("1 2 3")),
        );

        parse_test(
            StringArg::GreedyPhrase,
            &mut (),
            &(),
            &mut StrReader::new("cyrillic фа"),
            11,
            Ok(Cow::Borrowed("cyrillic фа")),
        );
    }
}
