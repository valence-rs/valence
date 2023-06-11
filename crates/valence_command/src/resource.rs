use valence_core::protocol::packet::command::Parser;

use crate::parser::{BrigadierArgument, NoParsingBuild, Parse, ParsingPurpose, ParsingResult};
use crate::reader::StrReader;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResourceLocation<'a>(pub &'a str);

impl<'a> Parse<'a> for ResourceLocation<'a> {
    type Data = ();

    type Error = NoParsingBuild;

    type Suggestions = NoParsingBuild;

    fn parse(
        _data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        _purpose: ParsingPurpose,
    ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
        ParsingResult {
            suggestions: None,
            result: Ok(Some(Self(reader.read_resource_location_str()))),
        }
    }
}

impl<'a> BrigadierArgument<'a> for ResourceLocation<'a> {
    fn parser(_data: Option<&Self::Data>) -> Parser<'a> {
        Parser::ResourceLocation
    }
}
