use std::any::TypeId;

use valence_core::text::Text;
use valence_core::translation_key::{
    ARGUMENT_FLOAT_BIG, ARGUMENT_FLOAT_LOW, ARGUMENT_INTEGER_BIG, ARGUMENT_INTEGER_LOW,
    PARSING_FLOAT_EXPECTED, PARSING_FLOAT_INVALID, PARSING_INT_EXPECTED, PARSING_INT_INVALID,
};

use crate::parse::{Parse, ParseResult};
use crate::pkt;
use crate::reader::StrReader;
use crate::suggestions::RawParseSuggestions;

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
        impl<'a> Parse<'a> for $ty {
            type Data = NumberBounds<Self>;

            type Suggestions = ();

            fn id() -> TypeId {
                TypeId::of::<Self>()
            }

            fn parse(
                data: &Self::Data,
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

            fn brigadier(data: &Self::Data) -> Option<pkt::Parser<'static>> {
                Some(pkt::Parser::$parser {
                    min: data.min,
                    max: data.max,
                })
            }

            fn vanilla(_data: &Self::Data) -> bool {
                true
            }
        }

        impl<'a> RawParseSuggestions<'a> for $ty {
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
