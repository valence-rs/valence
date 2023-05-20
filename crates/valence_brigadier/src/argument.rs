use std::borrow::Cow;

use serde_json::de::StrRead;
use serde_json::StreamDeserializer;
use valence_core::game_mode::GameMode;
use valence_core::packet::s2c::play::command_tree::{Parser, StringArg};
use valence_core::text::Text;
use valence_core::translation_key::{
    ARGUMENT_COMPONENT_INVALID, ARGUMENT_FLOAT_BIG, ARGUMENT_FLOAT_LOW, ARGUMENT_GAMEMODE_INVALID,
    ARGUMENT_INTEGER_BIG, ARGUMENT_INTEGER_LOW, COMMAND_UNKNOWN_ARGUMENT, PARSING_BOOL_EXPECTED,
    PARSING_BOOL_INVALID, PARSING_FLOAT_EXPECTED, PARSING_FLOAT_INVALID, PARSING_INT_EXPECTED,
    PARSING_INT_INVALID, PARSING_QUOTE_EXPECTED_END,
};

use crate::parser::{BrigadierArgument, DefaultParsableData, ErrorMessage, Parsable, ParsingError};
use crate::reader::StrReader;

macro_rules! num_impl {
    ($($ty:ty, $float:expr, $parser:ident,)*) => {
        $(impl<'a> Parsable<'a> for $ty {
            type Data = (Self, Self);

            fn parse(data: &Self::Data, reader: &mut StrReader<'a>) -> Result<Self, ParsingError> {
                let num_str = reader.read_num::<$float>().ok_or_else(|| {
                    if $float {
                        PARSING_FLOAT_EXPECTED
                    } else {
                        PARSING_INT_EXPECTED
                    }
                    .empty()
                })?;

                let num = num_str.parse().map_err(|_| {
                    if $float {
                        PARSING_FLOAT_INVALID
                    } else {
                        PARSING_INT_INVALID
                    }
                    .empty()
                })?;

                if data.0 > num {
                    Err(if $float {
                        ARGUMENT_FLOAT_LOW
                    } else {
                        ARGUMENT_INTEGER_LOW
                    }
                    .with(vec![
                        data.0.to_string().into(),
                        num_str.to_string().into(),
                    ]))?
                }

                if data.1 < num {
                    Err(if $float {
                        ARGUMENT_FLOAT_BIG
                    } else {
                        ARGUMENT_INTEGER_BIG
                    }
                    .with(vec![
                        data.1.to_string().into(),
                        num_str.to_string().into(),
                    ]))?
                }

                Ok(num)
            }
        }

        impl<'a> BrigadierArgument<'a> for $ty {
            fn brigadier_parser(data: &Self::Data) -> Parser<'a> {
                let min = Some(data.0).filter(|v| *v != Self::MIN);
                let max = Some(data.1).filter(|v| *v != Self::MAX);
                Parser::$parser { min, max }
            }
        }

        impl<'a> DefaultParsableData<'a> for $ty {
            const DEFAULT_DATA: Self::Data = (<$ty>::MIN, <$ty>::MAX);
        })*
    };
}

num_impl!(i32, false, Integer, i64, false, Long, f32, true, Float, f64, true, Double,);

impl<'a> Parsable<'a> for bool {
    type Data = ();

    fn parse(_data: &Self::Data, reader: &mut StrReader<'a>) -> Result<Self, ParsingError> {
        match reader.read_unquoted_str() {
            Some("true") | Some("1") => Ok(true),
            Some("false") | Some("0") => Ok(false),
            Some(other) => Err(PARSING_BOOL_INVALID.with(vec![other.to_string().into()]))?,
            None => Err(PARSING_BOOL_EXPECTED.empty())?,
        }
    }
}

impl<'a> BrigadierArgument<'a> for bool {
    fn brigadier_parser(_data: &<Self as Parsable<'a>>::Data) -> Parser<'a> {
        Parser::Bool
    }
}

impl<'a> DefaultParsableData<'a> for bool {
    const DEFAULT_DATA: <Self as Parsable<'a>>::Data = ();
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InclusiveRange<T> {
    pub min: T,
    pub max: T,
}

macro_rules! inclusive_range_impl {
    ($($num_ty:ty, $float:expr, $parser:ident,)*) => {
        $(impl Default for InclusiveRange<$num_ty> {
            fn default() -> Self {
                Self {
                    min: <$num_ty>::MIN,
                    max: <$num_ty>::MAX,
                }
            }
        }

        impl<'a> Parsable<'a> for InclusiveRange<$num_ty> {
            type Data = [<$num_ty as Parsable<'a>>::Data; 2];

            fn parse(data: &Self::Data, reader: &mut StrReader<'a>) -> Result<Self, ParsingError> {
                let min = if $float {
                    let mut any_dot = false;
                    let begin = reader.cursor();
                    while let Some(ch) = reader.peek_char() {
                        match ch {
                            '.' if reader.peek_char_offset(1) == Some('.') => break,
                            '.' if !any_dot => any_dot = true,
                            '0'..='9' | '+' | '-' => {}
                            _ => Err(COMMAND_UNKNOWN_ARGUMENT.empty())?,
                        }
                        let _ = reader.next_char();
                    }
                    match reader.str_from_to(begin, reader.cursor()) {
                        Some("") | None => <$num_ty>::MIN,
                        Some(str) => str.parse().map_err(|_| COMMAND_UNKNOWN_ARGUMENT.empty())?
                    }
                } else {
                    match reader.peek_char() {
                        Some('0'..='9') | Some('+') | Some('-') => <$num_ty>::parse(&data[0], reader)?,
                        _ => data[0].0,
                    }
                };

                Ok(if reader.remaining_str().starts_with("..") {
                    reader.skip_chars(2);
                    let max = match reader.peek_char() {
                        Some('0'..='9') | Some('-') | Some('+') => <$num_ty>::parse(&data[1], reader)?,
                        Some('.') if $float => <$num_ty>::parse(&data[1], reader)?,
                        _ => data[1].1,
                    };

                    if max < min {
                        Err(COMMAND_UNKNOWN_ARGUMENT.empty())?;
                    }

                    Self { min, max }
                } else {
                    Self {
                        min,
                        max: min
                    }
                })
            }
        }

        impl<'a> BrigadierArgument<'a> for InclusiveRange<$num_ty> {
            fn brigadier_parser(_data: &Self::Data) -> Parser<'a> {
                Parser::$parser
            }
        }

        impl<'a> DefaultParsableData<'a> for InclusiveRange<$num_ty> {
            const DEFAULT_DATA: Self::Data = [<$num_ty>::DEFAULT_DATA, <$num_ty>::DEFAULT_DATA];
        }

        )*
    };
}

inclusive_range_impl!(
    i32, false, IntRange, i64, false, IntRange, f32, true, FloatRange, f64, true, FloatRange,
);

impl<'a> Parsable<'a> for &'a str {
    type Data = StringArg;

    fn parse(data: &Self::Data, reader: &mut StrReader<'a>) -> Result<Self, ParsingError> {
        match data {
            StringArg::SingleWord => Ok(reader.read_unquoted_str().unwrap_or("")),
            StringArg::QuotablePhrase => match reader.peek_char() {
                Some('"') | Some('\'') => {
                    reader.next_char();
                    Ok(reader
                        .read_quoted_str()
                        .ok_or_else(|| PARSING_QUOTE_EXPECTED_END.empty())?)
                }
                _ => Ok(reader.read_unquoted_str().unwrap_or("")),
            },
            StringArg::GreedyPhrase => {
                let result = reader.remaining_str();
                reader.cursor_to_end();
                Ok(result)
            }
        }
    }
}

impl<'a> BrigadierArgument<'a> for &'a str {
    fn brigadier_parser(data: &<Self as Parsable<'a>>::Data) -> Parser<'a> {
        Parser::String(*data)
    }
}

impl<'a> Parsable<'a> for String {
    type Data = StringArg;

    fn parse(data: &Self::Data, reader: &mut StrReader<'a>) -> Result<Self, ParsingError> {
        <&'a str>::parse(data, reader).map(|s| s.into())
    }
}

impl<'a> BrigadierArgument<'a> for String {
    fn brigadier_parser(data: &<Self as Parsable<'a>>::Data) -> Parser<'a> {
        <&'a str>::brigadier_parser(data)
    }
}

impl<'a> Parsable<'a> for Cow<'a, str> {
    type Data = StringArg;

    fn parse(data: &Self::Data, reader: &mut StrReader<'a>) -> Result<Self, ParsingError> {
        <&'a str>::parse(data, reader).map(Cow::Borrowed)
    }
}

impl<'a> BrigadierArgument<'a> for Cow<'a, str> {
    fn brigadier_parser(data: &<Self as Parsable<'a>>::Data) -> Parser<'a> {
        <&'a str>::brigadier_parser(data)
    }
}

impl<'a> Parsable<'a> for GameMode {
    type Data = ();

    fn parse(_data: &Self::Data, reader: &mut StrReader<'a>) -> Result<Self, ParsingError> {
        let value = reader.read_unquoted_str().unwrap_or("");
        match value.to_ascii_lowercase().as_str() {
            "survival" | "0" => Ok(Self::Survival),
            "creative" | "1" => Ok(Self::Creative),
            "adventure" | "2" => Ok(Self::Adventure),
            "spectator" | "3" => Ok(Self::Spectator),
            _ => Err(ARGUMENT_GAMEMODE_INVALID.with(vec![value.to_string().into()]))?,
        }
    }
}

impl<'a> BrigadierArgument<'a> for GameMode {
    fn brigadier_parser(_data: &<Self as Parsable<'a>>::Data) -> Parser<'a> {
        Parser::GameMode
    }
}

impl<'a> Parsable<'a> for Text {
    type Data = ();

    fn parse(_data: &Self::Data, reader: &mut StrReader<'a>) -> Result<Self, ParsingError> {
        let rem_str = reader.remaining_str();
        let mut stream = StreamDeserializer::new(StrRead::new(rem_str));
        let result = match stream.next() {
            Some(Ok(value)) => Ok(value),
            _ => Err(ARGUMENT_COMPONENT_INVALID
                .with(vec![rem_str
                    .get(..stream.byte_offset())
                    .unwrap()
                    .to_string()
                    .into()])
                .into()),
        };
        unsafe { reader.set_cursor(reader.cursor() + stream.byte_offset()) };
        result
    }
}

impl<'a> BrigadierArgument<'a> for Text {
    fn brigadier_parser(_data: &<Self as Parsable<'a>>::Data) -> Parser<'a> {
        Parser::Component
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn num_test() {
        let mut reader = StrReader::new("10 20 20. 30.1 10..20 ..20 30.. 40.0..50.");
        assert_eq!(i32::parse(&(i32::MIN, i32::MAX), &mut reader).unwrap(), 10);
        assert_eq!(reader.next_char().unwrap(), ' ');
        assert_eq!(i64::parse(&(i64::MIN, i64::MAX), &mut reader).unwrap(), 20);
        assert_eq!(reader.next_char().unwrap(), ' ');
        assert_eq!(f32::parse(&(f32::MIN, f32::MAX), &mut reader).unwrap(), 20.);
        assert_eq!(reader.next_char().unwrap(), ' ');
        assert_eq!(
            f64::parse(&(f64::MIN, f64::MAX), &mut reader).unwrap(),
            30.1
        );
        assert_eq!(reader.next_char().unwrap(), ' ');
        assert_eq!(
            InclusiveRange::<i32>::parse(
                &[(i32::MIN, i32::MAX), (i32::MIN, i32::MAX)],
                &mut reader
            )
            .unwrap(),
            InclusiveRange { min: 10, max: 20 }
        );
        assert_eq!(reader.next_char().unwrap(), ' ');
        assert_eq!(
            InclusiveRange::<i32>::parse(
                &[(i32::MIN, i32::MAX), (i32::MIN, i32::MAX)],
                &mut reader
            )
            .unwrap(),
            InclusiveRange {
                min: i32::MIN,
                max: 20
            }
        );
        assert_eq!(reader.next_char().unwrap(), ' ');
        assert_eq!(
            InclusiveRange::<i32>::parse(
                &[(i32::MIN, i32::MAX), (i32::MIN, i32::MAX)],
                &mut reader
            )
            .unwrap(),
            InclusiveRange {
                min: 30,
                max: i32::MAX
            }
        );
        assert_eq!(reader.next_char().unwrap(), ' ');
        assert_eq!(
            InclusiveRange::<f32>::parse(
                &[(f32::MIN, f32::MAX), (f32::MIN, f32::MAX)],
                &mut reader
            )
            .unwrap(),
            InclusiveRange {
                min: 40.0,
                max: 50.
            }
        );
        assert_eq!(reader.next_char(), None);
    }

    #[test]
    fn str_test() {
        let mut reader = StrReader::new(r#"string "string2 string3" string4"#);
        assert_eq!(
            <&str>::parse(&StringArg::SingleWord, &mut reader).unwrap(),
            "string"
        );
        assert_eq!(reader.skip_only(' '), Some(()));
        assert_eq!(
            <&str>::parse(&StringArg::QuotablePhrase, &mut reader).unwrap(),
            "string2 string3"
        );
        assert_eq!(reader.skip_only(' '), Some(()));
        assert_eq!(
            <&str>::parse(&StringArg::QuotablePhrase, &mut reader).unwrap(),
            "string4"
        );
    }
}
