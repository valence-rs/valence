use valence_core::protocol::packet::command::Parser;
use valence_core::text::Text;
use valence_core::translation_key::ARGUMENT_COMPONENT_INVALID;

use crate::parser::{BrigadierArgument, ParsingError, ParsingResult};

#[macro_export]
macro_rules! parsable_json {
    ($ty:ty, $func: ident) => {
        paste::paste!(
            #[derive(Clone, Copy, Debug, PartialEq)]
            pub struct [<$ty JsonError>]<'a> (&'a str);

            impl<'a> $crate::parser::ParsingBuild<$crate::parser::ParsingError> for [<$ty JsonError>] <'a> {
                fn build(self) -> $crate::parser::ParsingError {
                    $func(self.0)
                }
            }

            impl<'a> $crate::parser::Parsable<'a> for $ty {
                type Data = ();

                type Suggestions = $crate::parser::NoParsingBuild;

                type Error = [<$ty JsonError>] <'a>;

                fn parse(
                    _data: std::option::Option<&Self::Data>,
                    reader: &mut $crate::reader::StrReader<'a>,
                    _purpose: $crate::parser::ParsingPurpose,
                ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
                    let rem_str = reader.remaining_str();
                    let mut stream = serde_json::StreamDeserializer::new(serde_json::de::StrRead::new(rem_str));
                    let result = match stream.next() {
                        std::option::Option::Some(std::result::Result::Ok(value)) =>
                            std::result::Result::Ok(std::option::Option::Some(value)),
                        _ => std::result::Result::Err(()),
                    };

                    let begin = reader.cursor();
                    unsafe { reader.set_cursor(begin + stream.byte_offset()) };

                    $crate::parser::ParsingResult {
                        suggestions: std::option::Option::None,
                        result: result.map_err(|_| {
                            (
                                begin..reader.cursor(),
                                [<$ty JsonError>] (
                                    reader.str().get(begin..reader.cursor()).unwrap()
                                ),
                            )
                        }),
                    }
                }
            }
        );
    };
}

fn component_err(str: &str) -> ParsingError {
    ParsingError::translate(ARGUMENT_COMPONENT_INVALID, vec![str.to_string().into()])
}

parsable_json!(Text, component_err);

impl<'a> BrigadierArgument<'a> for Text {
    fn parser(_data: Option<&Self::Data>) -> Parser<'a> {
        Parser::Component
    }
}

#[cfg(test)]
mod tests {

    use valence_core::text::{Color, TextFormat};

    use super::*;
    use crate::parser::{Parsable, ParsingPurpose};
    use crate::reader::StrReader;

    #[test]
    fn component_test() {
        assert_eq!(
            Text::parse(
                None,
                &mut StrReader::new(r#"{"text":"hello","color":"red"}"#),
                ParsingPurpose::Reading
            ),
            ParsingResult {
                suggestions: None,
                result: Ok(Some(Text::text("hello").color(Color::RED)))
            }
        )
    }
}
