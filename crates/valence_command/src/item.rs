use valence_core::item::ItemKind;
use valence_core::protocol::packet::command::Parser;
use valence_core::translation_key::ARGUMENT_ITEM_ID_INVALID;
use valence_nbt::Compound;

use crate::p_try;
use crate::parser::{
    BrigadierArgument, Parse, ParsingBuild, ParsingError, ParsingPurpose, ParsingResult,
    ParsingSuggestions,
};
use crate::reader::StrReader;

#[derive(Clone, Debug, PartialEq)]
pub struct ItemPredicate<'a> {
    pub id: ItemPredicateId<'a>,
    pub nbt: Option<Compound>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ItemPredicateId<'a> {
    Tag(&'a str),
    Kind(ItemKind),
}

#[derive(Clone, Copy)]
pub enum ItemError {
    Kind,
}

impl ParsingBuild<ParsingError> for ItemError {
    fn build(self) -> ParsingError {
        match self {
            Self::Kind => ParsingError::translate(ARGUMENT_ITEM_ID_INVALID, vec![]),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ItemSuggestion {
    Kind,
}

impl<'a> ParsingBuild<ParsingSuggestions<'a>> for ItemSuggestion {
    fn build(self) -> ParsingSuggestions<'a> {
        match self {
            Self::Kind => ParsingSuggestions::Borrowed(&[]),
        }
    }
}

fn read_kind(reader: &mut StrReader) -> ParsingResult<ItemKind, ItemSuggestion, ItemError> {
    let begin = reader.cursor();
    match ItemKind::from_str(reader.read_unquoted_str()) {
        Some(k) => ParsingResult {
            suggestions: None,
            result: Ok(Some(k)),
        },
        None => ParsingResult {
            suggestions: Some((begin..reader.cursor(), ItemSuggestion::Kind)),
            result: Err((begin..reader.cursor(), ItemError::Kind)),
        },
    }
}

impl<'a> Parse<'a> for ItemPredicate<'a> {
    type Data = ();

    type Suggestions = ItemSuggestion;

    type Error = ItemError;

    fn parse(
        _data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        _purpose: ParsingPurpose,
    ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
        let id = if reader.skip_char('#') {
            ItemPredicateId::Tag(reader.read_resource_location_str())
        } else {
            ItemPredicateId::Kind(p_try!(read_kind(reader)).1.unwrap())
        };

        // TODO: nbt

        ParsingResult {
            suggestions: None,
            result: Ok(Some(Self { id, nbt: None })),
        }
    }
}

impl<'a> BrigadierArgument<'a> for ItemPredicate<'a> {
    fn parser(_data: Option<&Self::Data>) -> Parser<'a> {
        Parser::ItemPredicate
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ItemStackArgument {
    pub kind: ItemKind,
    pub nbt: Option<Compound>,
}

impl<'a> Parse<'a> for ItemStackArgument {
    type Data = ();

    type Error = ItemError;

    type Suggestions = ItemSuggestion;

    fn parse(
        _data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        _purpose: ParsingPurpose,
    ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
        ParsingResult {
            suggestions: None,
            result: Ok(Some(Self {
                kind: p_try!(read_kind(reader)).1.unwrap(),
                nbt: None,
            })),
        }
    }
}

impl<'a> BrigadierArgument<'a> for ItemStackArgument {
    fn parser(_data: Option<&Self::Data>) -> Parser<'a> {
        Parser::ItemStack
    }
}
