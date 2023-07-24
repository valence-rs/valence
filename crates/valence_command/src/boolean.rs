use std::any::TypeId;

use valence_core::text::Text;
use valence_core::translation_key::{PARSING_BOOL_EXPECTED, PARSING_BOOL_INVALID};

use crate::parse::{Parse, ParseResult};
use crate::pkt;
use crate::reader::{StrLocated, StrReader};
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
        reader.err_located(|reader| match reader.read_unquoted_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            o if o.is_empty() => Err(Text::translate(PARSING_BOOL_EXPECTED, vec![])),
            o => Err(Text::translate(
                PARSING_BOOL_INVALID,
                vec![o.to_string().into()],
            )),
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
