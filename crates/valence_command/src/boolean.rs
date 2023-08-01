use std::any::TypeId;

use valence_core::text::Text;
use valence_core::translation_key::{PARSING_BOOL_EXPECTED, PARSING_BOOL_INVALID};

use crate::parse::{Parse, ParseResult};
use crate::pkt;
use crate::reader::StrReader;
use crate::suggestions::RawParseSuggestions;

impl<'a> Parse<'a> for bool {
    type Data = ();

    type Suggestions = ();

    fn id() -> TypeId {
        TypeId::of::<Self>()
    }

    fn parse(
        _data: &Self::Data,
        _suggestions: &mut Self::Suggestions,
        reader: &mut StrReader<'a>,
    ) -> ParseResult<Self> {
        reader.err_located(|reader| {
            let str = reader.read_unquoted_str(); 
            if str.is_empty() {
                Err(Text::translate(PARSING_BOOL_EXPECTED, vec![]))
            } else if str.eq_ignore_ascii_case("true") {
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

    fn brigadier(data: &Self::Data) -> Option<pkt::Parser<'static>> {
        Some(pkt::Parser::Bool)
    }

    fn vanilla(data: &Self::Data) -> bool {
        true
    }
}

impl<'a> RawParseSuggestions<'a> for bool {
    fn call_suggestions(
        data: &Self::Data,
        real: crate::command::RealCommandExecutor,
        transaction: crate::suggestions::SuggestionsTransaction,
        executor: crate::command::CommandExecutor,
        answer: &mut crate::suggestions::SuggestionAnswerer,
        suggestions: Self::Suggestions,
        command: String,
        world: &bevy_ecs::world::World,
    ) {
        todo!()
    }
}
