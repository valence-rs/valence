use std::any::TypeId;
use std::borrow::Cow;

use bevy_ecs::system::SystemParamItem;
use valence_core::text::Text;
use valence_core::translation_key::{PARSING_BOOL_EXPECTED, PARSING_BOOL_INVALID};

use crate::command::CommandExecutorBase;
use crate::nodes::NodeSuggestion;
use crate::parse::{Parse, ParseResult};
use crate::pkt;
use crate::reader::{ArcStrReader, StrLocated, StrReader, StrSpan};
use crate::suggestions::Suggestion;

#[async_trait::async_trait]
impl Parse for bool {
    type Item<'a> = bool;

    type Data<'a> = ();

    type Suggestions = StrSpan;

    type SuggestionsParam = ();

    type SuggestionsAsyncData = Cow<'static, [Suggestion<'static>]>;

    const VANILLA: bool = true;

    fn parse_id() -> TypeId {
        TypeId::of::<Self>()
    }

    fn item_id() -> TypeId {
        TypeId::of::<Self>()
    }

    fn parse<'a>(
        _data: &Self::Data<'a>,
        suggestions: &mut Self::Suggestions,
        reader: &mut StrReader<'a>,
    ) -> ParseResult<Self> {
        reader.span_err_located(suggestions, |reader| {
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

    fn brigadier(_data: &Self::Data<'_>) -> Option<pkt::Parser<'static>> {
        Some(pkt::Parser::Bool)
    }

    fn brigadier_suggestions(_data: &Self::Data<'_>) -> Option<NodeSuggestion> {
        None
    }

    fn create_suggestions_data(
        _data: &Self::Data<'_>,
        command: ArcStrReader,
        _executor: CommandExecutorBase,
        suggestions: &Self::Suggestions,
        _param: SystemParamItem<Self::SuggestionsParam>,
    ) -> Self::SuggestionsAsyncData {
        const EMPTY: &[Suggestion<'static>] = &[];
        const ONLY_TRUE: &[Suggestion<'static>] = &[Suggestion::new_str("true")];
        const ONLY_FALSE: &[Suggestion<'static>] = &[Suggestion::new_str("false")];
        const BOTH: &[Suggestion<'static>] =
            &[Suggestion::new_str("true"), Suggestion::new_str("false")];

        let str = suggestions.in_str(command.reader().str()).unwrap();

        Cow::Borrowed(if str.len() > 5 {
            EMPTY
        } else {
            let lc_str = str.to_ascii_lowercase();
            if str.len() == 0 {
                BOTH
            } else if "true".starts_with(&lc_str) {
                ONLY_TRUE
            } else if "false".starts_with(&lc_str) {
                ONLY_FALSE
            } else {
                EMPTY
            }
        })
    }

    async fn suggestions(
        _command: ArcStrReader,
        _executor: CommandExecutorBase,
        suggestions: Box<Self::Suggestions>,
        async_data: Self::SuggestionsAsyncData,
    ) -> StrLocated<Cow<'static, [Suggestion<'static>]>> {
        StrLocated::new(*suggestions, async_data)
    }
}
