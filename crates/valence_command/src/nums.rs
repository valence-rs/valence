use std::any::TypeId;
use std::borrow::Cow;

use bevy_ecs::system::SystemParamItem;
use valence_core::text::Text;
use valence_core::translation_key::{
    ARGUMENT_FLOAT_BIG, ARGUMENT_FLOAT_LOW, ARGUMENT_INTEGER_BIG, ARGUMENT_INTEGER_LOW,
    PARSING_FLOAT_EXPECTED, PARSING_FLOAT_INVALID, PARSING_INT_EXPECTED, PARSING_INT_INVALID,
};

use crate::command::CommandExecutorBase;
use crate::nodes::NodeSuggestion;
use crate::parse::{Parse, ParseResult};
use crate::pkt;
use crate::reader::{ArcStrReader, StrLocated, StrReader, StrSpan};
use crate::suggestions::Suggestion;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumberBounds<T> {
    pub min: Option<T>,
    pub max: Option<T>,
}

impl<T> Default for NumberBounds<T> {
    fn default() -> Self {
        Self {
            min: None,
            max: None,
        }
    }
}

macro_rules! num_parse {
    ($ty:ty, $parser:ident, $low:expr, $big:expr, $expected:expr, $invalid:expr) => {
        #[async_trait::async_trait]
        impl Parse for $ty {
            type Item<'a> = Self;

            type Data<'a> = NumberBounds<Self>;

            type Suggestions = ();

            type SuggestionsAsyncData = ();

            type SuggestionsParam = ();

            const VANILLA: bool = true;

            fn parse_id() -> TypeId {
                TypeId::of::<Self>()
            }

            fn item_id() -> TypeId {
                TypeId::of::<Self>()
            }

            fn parse<'a>(
                data: &Self::Data<'a>,
                _suggestions: &mut Self::Suggestions,
                reader: &mut StrReader<'a>,
            ) -> ParseResult<Self> {
                reader.err_located(|reader| {
                    let num_str = reader.read_num_str();
                    if num_str.is_empty() {
                        Err(Text::translate($expected, vec![]))
                    } else {
                        match (num_str.parse::<Self>(), data.min, data.max) {
                            (Ok(num), Some(min), _) if num < min => Err(Text::translate(
                                $low,
                                vec![min.to_string().into(), num_str.to_string().into()],
                            )),
                            (Ok(num), _, Some(max)) if num > max => Err(Text::translate(
                                $big,
                                vec![max.to_string().into(), num_str.to_string().into()],
                            )),
                            (Ok(num), ..) => Ok(num),
                            (Err(_), ..) => {
                                Err(Text::translate($invalid, vec![num_str.to_string().into()]))
                            }
                        }
                    }
                })
            }

            fn brigadier(data: &Self::Data<'_>) -> Option<pkt::Parser<'static>> {
                Some(pkt::Parser::$parser {
                    min: data.min,
                    max: data.max,
                })
            }

            fn brigadier_suggestions(_data: &Self::Data<'_>) -> Option<NodeSuggestion> {
                None
            }

            /// Creates a data which will be passed then to
            /// [`Parse::suggestions`] method
            fn create_suggestions_data(
                _data: &Self::Data<'_>,
                _command: ArcStrReader,
                _executor: CommandExecutorBase,
                _suggestion: &Self::Suggestions,
                _param: SystemParamItem<Self::SuggestionsParam>,
            ) -> Self::SuggestionsAsyncData {
                ()
            }

            async fn suggestions(
                _command: ArcStrReader,
                _executor: CommandExecutorBase,
                _suggestion: Box<Self::Suggestions>,
                _async_data: Self::SuggestionsAsyncData,
            ) -> StrLocated<Cow<'static, [Suggestion<'static>]>> {
                StrLocated::new(StrSpan::ZERO, Cow::Borrowed(&[]))
            }
        }
    };
}

num_parse!(
    i32,
    Integer,
    ARGUMENT_INTEGER_LOW,
    ARGUMENT_INTEGER_BIG,
    PARSING_INT_EXPECTED,
    PARSING_INT_INVALID
);

num_parse!(
    i64,
    Long,
    ARGUMENT_INTEGER_LOW,
    ARGUMENT_INTEGER_BIG,
    PARSING_INT_EXPECTED,
    PARSING_INT_INVALID
);

num_parse!(
    f32,
    Float,
    ARGUMENT_FLOAT_LOW,
    ARGUMENT_FLOAT_BIG,
    PARSING_FLOAT_EXPECTED,
    PARSING_FLOAT_INVALID
);

num_parse!(
    f64,
    Double,
    ARGUMENT_FLOAT_LOW,
    ARGUMENT_FLOAT_BIG,
    PARSING_FLOAT_EXPECTED,
    PARSING_FLOAT_INVALID
);
