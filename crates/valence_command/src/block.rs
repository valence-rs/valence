use valence_block::{BlockKind, BlockState, PropName, PropValue};
use valence_core::protocol::packet::command::Parser;
use valence_core::translation_key::{
    ARGUMENT_BLOCK_ID_INVALID, ARGUMENT_BLOCK_PROPERTY_DUPLICATE, ARGUMENT_BLOCK_PROPERTY_INVALID,
    ARGUMENT_BLOCK_PROPERTY_NOVALUE, ARGUMENT_BLOCK_PROPERTY_UNCLOSED,
    ARGUMENT_BLOCK_PROPERTY_UNKNOWN,
};
use valence_nbt::Compound;

use crate::parser::{
    BrigadierArgument, Parsable, ParsingBuild, ParsingError, ParsingPurpose, ParsingResult,
    ParsingSuggestions, Suggestion,
};
use crate::reader::StrReader;
use crate::{parsing_ret_err, parsing_token};

#[derive(Clone, Debug, PartialEq)]
pub struct BlockPredicate<'a> {
    pub kind: BlockPredicateKind<'a>,
    pub states: Vec<(PropName, PropValue)>,
    pub tags: Option<Compound>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlockPredicateKind<'a> {
    Tag(&'a str),
    Kind(BlockKind),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlockError<'a> {
    Kind(&'a str),
    PropInvalid {
        kind: &'a str,
        name: &'a str,
        value: &'a str,
    },
    PropDuplicate {
        kind: &'a str,
        name: &'a str,
    },
    PropNoValue {
        kind: &'a str,
        name: &'a str,
    },
    PropUnknown {
        kind: &'a str,
        name: &'a str,
    },
    PropUnclosed,
}

impl<'a> ParsingBuild<ParsingError> for BlockError<'a> {
    fn build(self) -> ParsingError {
        match self {
            Self::Kind(kind) => {
                ParsingError::translate(ARGUMENT_BLOCK_ID_INVALID, vec![kind.to_string().into()])
            }
            Self::PropInvalid { kind, name, value } => ParsingError::translate(
                ARGUMENT_BLOCK_PROPERTY_INVALID,
                vec![
                    kind.to_string().into(),
                    value.to_string().into(),
                    name.to_string().into(),
                ],
            ),
            Self::PropDuplicate { kind, name } => ParsingError::translate(
                ARGUMENT_BLOCK_PROPERTY_DUPLICATE,
                vec![name.to_string().into(), kind.to_string().into()],
            ),
            Self::PropNoValue { kind, name } => ParsingError::translate(
                ARGUMENT_BLOCK_PROPERTY_NOVALUE,
                vec![name.to_string().into(), kind.to_string().into()],
            ),
            Self::PropUnknown { kind, name } => ParsingError::translate(
                ARGUMENT_BLOCK_PROPERTY_UNKNOWN,
                vec![kind.to_string().into(), name.to_string().into()],
            ),
            Self::PropUnclosed => ParsingError::translate(ARGUMENT_BLOCK_PROPERTY_UNCLOSED, vec![]),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BlockSuggestions {
    BlockKind,
    PropName,
    PropValue,
    StateTagBegin,
    TagBegin,
    EqualSign,
    StateEnd,
    TagEnd,
}

macro_rules! bp_names {
    ($n:ident, $i:ident) => {
        #[ctor::ctor]
        static $n: [Suggestion<'static>; $i::ALL.len()] =
            { $i::ALL.map(|v| Suggestion::new_str(v.to_str())) };
    };
}

bp_names!(BLOCK_KINDS, BlockKind);
bp_names!(PROP_NAMES, PropName);
bp_names!(PROP_VALUES, PropValue);

const STATE_TAG_BEGIN: &'static [Suggestion<'static>] =
    &[Suggestion::new_str("["), Suggestion::new_str("{")];

const TAG_BEGIN: &'static [Suggestion<'static>] = &[Suggestion::new_str("{")];

const EQ_SIGN: &'static [Suggestion<'static>] = &[Suggestion::new_str("=")];

const TAG_END: &'static [Suggestion<'static>] =
    &[Suggestion::new_str(","), Suggestion::new_str("}")];

const STATE_END: &'static [Suggestion<'static>] =
    &[Suggestion::new_str(","), Suggestion::new_str("]")];

impl<'a> ParsingBuild<ParsingSuggestions<'a>> for BlockSuggestions {
    fn build(self) -> ParsingSuggestions<'a> {
        match self {
            Self::BlockKind => ParsingSuggestions::Borrowed(BLOCK_KINDS.as_slice()),
            Self::PropName => ParsingSuggestions::Borrowed(PROP_NAMES.as_slice()),
            Self::PropValue => ParsingSuggestions::Borrowed(PROP_VALUES.as_slice()),
            Self::StateTagBegin => ParsingSuggestions::Borrowed(STATE_TAG_BEGIN),
            Self::TagBegin => ParsingSuggestions::Borrowed(TAG_BEGIN),
            Self::EqualSign => ParsingSuggestions::Borrowed(EQ_SIGN),
            Self::TagEnd => ParsingSuggestions::Borrowed(TAG_END),
            Self::StateEnd => ParsingSuggestions::Borrowed(STATE_END),
        }
    }
}

impl<'a> Parsable<'a> for BlockPredicate<'a> {
    type Data = ();

    type Error = BlockError<'a>;

    type Suggestions = BlockSuggestions;

    fn parse(
        _data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        purpose: ParsingPurpose,
    ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
        let begin = reader.cursor();

        let kind = if reader.peek_char() == Some('#') {
            reader.next_char();
            BlockPredicateKind::Tag(reader.read_ident_str().1)
        } else {
            let kind_str = reader.read_ident_str().1;

            BlockPredicateKind::Kind(match BlockKind::from_str(kind_str) {
                Some(kind) => kind,
                None => {
                    return ParsingResult {
                        suggestions: Some((begin..reader.cursor(), BlockSuggestions::BlockKind)),
                        result: Err((begin..reader.cursor(), BlockError::Kind(kind_str))),
                    }
                }
            })
        };

        let kind_str = reader.str().get(begin..reader.cursor()).unwrap();

        let mut states = vec![];
        parsing_ret_err!(read_block_props(
            reader,
            purpose,
            kind_str,
            |prop_name, prop_value| {
                states.push((prop_name, prop_value));
                ParsingResult::ok()
            }
        ));

        ParsingResult {
            suggestions: None,
            result: Ok(match purpose {
                ParsingPurpose::Reading => Some(Self {
                    kind,
                    states,
                    tags: None,
                }),
                ParsingPurpose::Suggestion => None,
            }),
        }
    }
}

fn read_block_props<'a>(
    reader: &mut StrReader<'a>,
    purpose: ParsingPurpose,
    kind_str: &'a str,
    mut func: impl FnMut(PropName, PropValue) -> ParsingResult<(), BlockSuggestions, BlockError<'a>>,
) -> ParsingResult<(), BlockSuggestions, BlockError<'a>> {
    if reader.skip_char('[') {
        if !reader.skip_char(']') {
            loop {
                let res = <(PropName, PropValue)>::parse(Some(&kind_str), reader, purpose);

                match (res.result, purpose) {
                    (Ok(Some((name, value))), ParsingPurpose::Reading) => {
                        parsing_ret_err!(func(name, value));
                    }
                    (Ok(None), ParsingPurpose::Reading) => unreachable!(),
                    (Ok(_), ParsingPurpose::Suggestion) => {}
                    (Err(err), _) => {
                        return ParsingResult {
                            suggestions: res.suggestions,
                            result: Err(err),
                        };
                    }
                }

                reader.skip_char(' ');

                let begin = reader.cursor();

                match reader.next_char() {
                    Some(',') => {}
                    Some(']') => {
                        break;
                    }
                    _ => {
                        return ParsingResult {
                            suggestions: Some((begin..reader.cursor(), BlockSuggestions::StateEnd)),
                            result: Err((begin..reader.cursor(), BlockError::PropUnclosed)),
                        }
                    }
                };
            }
        }
    }
    ParsingResult::ok()
}

impl<'a> Parsable<'a> for (PropName, PropValue) {
    type Data = &'a str;

    type Error = BlockError<'a>;

    type Suggestions = BlockSuggestions;

    fn parse(
        kind_str: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        _purpose: ParsingPurpose,
    ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
        let kind_str = kind_str.unwrap_or(&"");
        let begin = reader.cursor();
        let prop_name_str = reader.read_unquoted_str();
        let prop_name = match PropName::from_str(prop_name_str) {
            Some(n) => n,
            None => {
                return ParsingResult {
                    suggestions: Some((begin..reader.cursor(), BlockSuggestions::PropName)),
                    result: Err((
                        begin..reader.cursor(),
                        BlockError::PropUnknown {
                            kind: kind_str,
                            name: prop_name_str,
                        },
                    )),
                }
            }
        };
        reader.skip_char(' ');
        parsing_token!(
            reader,
            '=',
            BlockError::PropNoValue {
                kind: kind_str,
                name: prop_name_str
            },
            BlockSuggestions::EqualSign,
        );
        reader.skip_char(' ');
        let begin = reader.cursor();
        let prop_value_str = reader.read_unquoted_str();
        let prop_value = match PropValue::from_str(prop_value_str) {
            Some(v) => v,
            None => {
                return ParsingResult {
                    suggestions: Some((begin..reader.cursor(), BlockSuggestions::PropValue)),
                    result: Err((
                        begin..reader.cursor(),
                        BlockError::PropInvalid {
                            kind: kind_str,
                            name: prop_name_str,
                            value: prop_value_str,
                        },
                    )),
                }
            }
        };

        ParsingResult {
            suggestions: None,
            result: Ok(Some((prop_name, prop_value))),
        }
    }
}

impl<'a> BrigadierArgument<'a> for BlockPredicate<'a> {
    fn parser(_data: Option<&Self::Data>) -> Parser<'a> {
        Parser::BlockPredicate
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlockStateArgument {
    pub state: BlockState,
    pub tags: Option<Compound>,
}

impl<'a> Parsable<'a> for BlockStateArgument {
    type Data = ();

    type Error = BlockError<'a>;

    type Suggestions = BlockSuggestions;

    fn parse(
        _data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        purpose: ParsingPurpose,
    ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
        let begin = reader.cursor();

        let kind = reader.read_ident_str().1;

        let kind_str = reader.str().get(begin..reader.cursor()).unwrap();

        let mut state = BlockState::from_kind(match BlockKind::from_str(kind) {
            Some(o) => o,
            None => {
                return ParsingResult {
                    suggestions: Some((begin..reader.cursor(), BlockSuggestions::BlockKind)),
                    result: Err((begin..reader.cursor(), BlockError::Kind(kind_str))),
                }
            }
        });

        parsing_ret_err!(read_block_props(
            reader,
            purpose,
            kind_str,
            |prop_name, prop_value| {
                state = state.set(prop_name, prop_value);
                ParsingResult::ok()
            }
        ));

        ParsingResult {
            suggestions: None,
            result: Ok(match purpose {
                ParsingPurpose::Reading => Some(Self { state, tags: None }),
                ParsingPurpose::Suggestion => None,
            }),
        }
    }
}

impl<'a> BrigadierArgument<'a> for BlockStateArgument {
    fn parser(_data: Option<&Self::Data>) -> Parser<'a> {
        Parser::BlockState
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_predicate_test() {
        assert_eq!(
            BlockPredicate::parse(
                None,
                &mut StrReader::new("chest[facing =north]"),
                ParsingPurpose::Reading
            ),
            ParsingResult {
                suggestions: None,
                result: Ok(Some(BlockPredicate {
                    kind: BlockPredicateKind::Kind(BlockKind::Chest),
                    states: vec![(PropName::Facing, PropValue::North)],
                    tags: None
                }))
            }
        );

        assert_eq!(
            BlockPredicate::parse(None, &mut StrReader::new("chest["), ParsingPurpose::Reading,),
            ParsingResult {
                suggestions: Some((6..6, BlockSuggestions::PropName)),
                result: Err((
                    6..6,
                    BlockError::PropUnknown {
                        kind: "chest",
                        name: ""
                    }
                )),
            }
        );
    }

    #[test]
    fn block_test() {
        assert_eq!(
            BlockStateArgument::parse(
                None,
                &mut StrReader::new("oak_slab[waterlogged = true]"),
                ParsingPurpose::Reading
            ),
            ParsingResult {
                suggestions: None,
                result: Ok(Some(BlockStateArgument {
                    state: BlockState::OAK_SLAB.set(PropName::Waterlogged, PropValue::True),
                    tags: None,
                }))
            }
        )
    }
}
