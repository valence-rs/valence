use valence_core::packet::s2c::play::command_tree::Parser;
use valence_core::text::Text;
use valence_core::translation_key::COMMAND_UNKNOWN_ARGUMENT;

use crate::reader::StrReader;

#[derive(Debug)]
pub enum ParsingError {
    User(Text),
    Internal(anyhow::Error),
}

impl From<Text> for ParsingError {
    fn from(value: Text) -> Self {
        Self::User(value)
    }
}

impl From<anyhow::Error> for ParsingError {
    fn from(value: anyhow::Error) -> Self {
        Self::Internal(value)
    }
}

pub trait ErrorMessage {
    fn empty(&self) -> Text;

    fn with(&self, with: impl Into<Vec<Text>>) -> Text;
}

impl ErrorMessage for &'static str {
    fn empty(&self) -> Text {
        Text::translate(*self, vec![])
    }

    fn with(&self, with: impl Into<Vec<Text>>) -> Text {
        Text::translate(*self, with)
    }
}

pub trait Parsable<'a>: Sized + 'a {
    type Data: 'a;

    fn parse(_data: &Self::Data, reader: &mut StrReader<'a>) -> Result<Self, ParsingError>;
}

pub trait BrigadierArgument<'a>: Parsable<'a> {
    fn brigadier_parser(_data: &<Self as Parsable<'a>>::Data) -> Parser<'a>;
}

pub trait DefaultParsableData<'a>: Parsable<'a> {
    const DEFAULT_DATA: <Self as Parsable<'a>>::Data;
}

pub fn parse_array_like<'a, const END: char, T: Parsable<'a>>(
    data: &T::Data,
    reader: &mut StrReader<'a>,
    into: &mut Vec<T>,
) -> Result<(), ParsingError> {
    while reader.skip_only(END).is_none() {
        into.push(T::parse(data, reader)?);
        reader.skip_recursive_only(' ');
        let ch = reader.next_char();
        if ch == Some(END) {
            break;
        } else if ch != Some(',') {
            Err(COMMAND_UNKNOWN_ARGUMENT.empty())?;
        }
        reader.skip_recursive_only(' ');
    }
    Ok(())
}
