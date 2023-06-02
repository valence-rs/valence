use std::borrow::Cow;
use std::mem::MaybeUninit;

use valence_block::{BlockKind, PropName, PropValue};
use valence_core::packet::s2c::play::command_tree::{Parser, StringArg};
use valence_core::translation_key::{
    ARGUMENT_ANGLE_INVALID, ARGUMENT_BLOCK_ID_INVALID, ARGUMENT_BLOCK_PROPERTY_DUPLICATE,
    ARGUMENT_BLOCK_PROPERTY_INVALID, ARGUMENT_BLOCK_PROPERTY_NOVALUE,
    ARGUMENT_BLOCK_PROPERTY_UNCLOSED, ARGUMENT_BLOCK_PROPERTY_UNKNOWN, ARGUMENT_DOUBLE_BIG,
    ARGUMENT_DOUBLE_LOW, ARGUMENT_FLOAT_BIG, ARGUMENT_FLOAT_LOW, ARGUMENT_INTEGER_BIG,
    ARGUMENT_INTEGER_LOW, ARGUMENT_LONG_BIG, ARGUMENT_LONG_LOW,
    ARGUMENT_POS_MIXED, COMMAND_EXPECTED_SEPARATOR, PARSING_BOOL_EXPECTED, PARSING_BOOL_INVALID,
    PARSING_DOUBLE_EXPECTED, PARSING_DOUBLE_INVALID, PARSING_FLOAT_EXPECTED, PARSING_FLOAT_INVALID,
    PARSING_INT_EXPECTED, PARSING_INT_INVALID, PARSING_LONG_EXPECTED, PARSING_LONG_INVALID,
    PARSING_QUOTE_EXPECTED_END,
};
use valence_nbt::Compound;

use crate::parser::{
    NoParsingBuild, Parsable, ParsingBuild, ParsingError, ParsingPurpose, ParsingResult,
    ParsingSuggestions, Suggestion,
};
use crate::parsing_error;
use crate::reader::StrReader;

pub trait BrigadierArgument<'a>: Parsable<'a> {
    fn parser(data: Option<&Self::Data>) -> Parser<'a>;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BoolParsingError<'a> {
    Expected,
    Invalid(&'a str),
}

impl<'a> ParsingBuild<ParsingError> for BoolParsingError<'a> {
    fn build(self) -> ParsingError {
        match self {
            Self::Expected => ParsingError::translate(PARSING_BOOL_EXPECTED, vec![]),
            Self::Invalid(given) => {
                ParsingError::translate(PARSING_BOOL_INVALID, vec![given.to_string().into()])
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoolSuggestions;

const BOOL_SUGGESTIONS: &'static [Suggestion<'static>] =
    &[Suggestion::new_str("true"), Suggestion::new_str("false")];

impl<'a> ParsingBuild<ParsingSuggestions<'a>> for BoolSuggestions {
    fn build(self) -> ParsingSuggestions<'a> {
        ParsingSuggestions::Borrowed(&BOOL_SUGGESTIONS)
    }
}

impl<'a> Parsable<'a> for bool {
    type Error = BoolParsingError<'a>;

    type Suggestions = BoolSuggestions;

    type Data = ();

    fn parse(
        _data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        _purpose: ParsingPurpose,
    ) -> ParsingResult<'a, Self> {
        let begin = reader.cursor();

        let result = match reader.read_unquoted_str() {
            "true" => Ok(true),
            "false" => Ok(false),
            "" if reader.is_ended() => Err(BoolParsingError::Expected),
            o => Err(BoolParsingError::Invalid(o)),
        };

        let pos = begin..reader.cursor();

        ParsingResult {
            suggestions: Some((pos.clone(), BoolSuggestions)),
            result: result.map(|v| Some(v)).map_err(|err| (pos, err)),
        }
    }
}

impl<'a> BrigadierArgument<'a> for bool {
    fn parser(_data: Option<&Self::Data>) -> Parser<'a> {
        Parser::Bool
    }
}

macro_rules! num_impl {
    ($($ty:ty, $error_n:ident, $parser: ident, $too_big:expr, $too_low:expr, $expected:expr, $invalid:expr,)*) => {
        $(#[derive(Clone, Copy, Debug, PartialEq)]
        pub enum $error_n<'a> {
            TooBig(&'a str, $ty),
            TooLow(&'a str, $ty),
            Invalid(&'a str),
            Expected,
        }

        impl<'a> ParsingBuild<ParsingError> for $error_n<'a> {
            fn build(self) -> ParsingError {
                match self {
                    Self::TooBig(given, bound) => {
                        ParsingError::translate($too_big, vec![bound.to_string().into(), given.to_string().into()])
                    }
                    Self::TooLow(given, bound) => {
                        ParsingError::translate($too_low, vec![bound.to_string().into(), given.to_string().into()])
                    }
                    Self::Invalid(given) => {
                        ParsingError::translate($invalid, vec![given.to_string().into()])
                    }
                    Self::Expected => ParsingError::translate($expected, vec![])
                }
            }
        }

        impl<'a> Parsable<'a> for $ty {
            type Error = $error_n<'a>;

            type Suggestions = NoParsingBuild;

            type Data = (Option<Self>, Option<Self>);

            fn parse(
                data: Option<&Self::Data>,
                reader: &mut StrReader<'a>,
                _purpose: ParsingPurpose,
            ) -> ParsingResult<'a, Self> {
                let begin = reader.cursor();

                let num_str = reader.read_num_str();

                let result = match (num_str.parse::<Self>(), data) {
                    (Ok(i), Some((Some(min), _))) if *min > i => {
                        Err($error_n::TooLow(num_str, *min))
                    },
                    (Ok(i), Some((_, Some(max)))) if *max < i => {
                        Err($error_n::TooBig(num_str, *max))
                    },
                    (Ok(i), _) => Ok(i),
                    (Err(_), _) if num_str.is_empty() && reader.is_ended() => Err($error_n::Expected),
                    (Err(_), _) => Err($error_n::Invalid(num_str))
                };

                ParsingResult {
                    suggestions: None,
                    result: result.map(|v| Some(v)).map_err(|err| (begin..reader.cursor(), err))
                }
            }
        }

        impl<'a> BrigadierArgument<'a> for $ty {
            fn parser(data: Option<&Self::Data>) -> Parser<'a> {
                let data = data.unwrap_or(&(None, None));
                Parser::$parser {
                    min: data.0,
                    max: data.1,
                }
            }
        })*
    };
}

num_impl!(
    i32,
    I32Error,
    Integer,
    ARGUMENT_INTEGER_BIG,
    ARGUMENT_INTEGER_LOW,
    PARSING_INT_EXPECTED,
    PARSING_INT_INVALID,
    i64,
    I64Error,
    Long,
    ARGUMENT_LONG_BIG,
    ARGUMENT_LONG_LOW,
    PARSING_LONG_EXPECTED,
    PARSING_LONG_INVALID,
    f32,
    F32Error,
    Float,
    ARGUMENT_FLOAT_BIG,
    ARGUMENT_FLOAT_LOW,
    PARSING_FLOAT_EXPECTED,
    PARSING_FLOAT_INVALID,
    f64,
    F64Error,
    Double,
    ARGUMENT_DOUBLE_BIG,
    ARGUMENT_DOUBLE_LOW,
    PARSING_DOUBLE_EXPECTED,
    PARSING_DOUBLE_INVALID,
);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SingleWordString<'a>(pub &'a str);

impl<'a> Parsable<'a> for SingleWordString<'a> {
    type Data = ();

    type Error = NoParsingBuild;

    type Suggestions = NoParsingBuild;

    fn parse(
        _data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        _purpose: ParsingPurpose,
    ) -> ParsingResult<'a, Self> {
        ParsingResult {
            suggestions: None,
            result: Ok(Some(SingleWordString(reader.read_unquoted_str()))),
        }
    }
}

impl<'a> BrigadierArgument<'a> for SingleWordString<'a> {
    fn parser(_data: Option<&Self::Data>) -> Parser<'a> {
        Parser::String(StringArg::SingleWord)
    }
}

parsing_error!(UnclosedQuoteError = PARSING_QUOTE_EXPECTED_END);

#[derive(Clone, Debug, PartialEq)]
pub struct QuotableString<'a>(pub Cow<'a, str>);

impl<'a> Parsable<'a> for QuotableString<'a> {
    type Data = ();

    type Error = UnclosedQuoteError;

    type Suggestions = NoParsingBuild;

    fn parse(
        _data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        purpose: ParsingPurpose,
    ) -> ParsingResult<'a, Self> {
        let begin = reader.cursor();

        let quoted = if reader.peek_char() == Some('"') {
            reader.next_char();
            true
        } else {
            false
        };

        let result = match purpose {
            ParsingPurpose::Reading => {
                if quoted {
                    reader
                        .read_started_quoted_str()
                        .map(|v| Some(Self(Cow::Owned(v))))
                        .ok_or_else(|| UnclosedQuoteError)
                } else {
                    Ok(Some(Self(Cow::Borrowed(reader.read_unquoted_str()))))
                }
            }
            ParsingPurpose::Suggestion => {
                if quoted {
                    if reader.skip_started_quoted_str() {
                        Ok(None)
                    } else {
                        Err(UnclosedQuoteError)
                    }
                } else {
                    reader.read_unquoted_str();
                    Ok(None)
                }
            }
        };

        ParsingResult {
            suggestions: None,
            result: result.map_err(|err| (begin..reader.cursor(), err)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GreedyString<'a>(pub &'a str);

impl<'a> Parsable<'a> for GreedyString<'a> {
    type Data = ();

    type Error = NoParsingBuild;

    type Suggestions = NoParsingBuild;

    fn parse(
        _data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        _purpose: ParsingPurpose,
    ) -> ParsingResult<'a, Self> {
        let result = reader.remaining_str();
        reader.to_end();
        ParsingResult {
            suggestions: None,
            result: Ok(Some(Self(result))),
        }
    }
}

impl<'a> BrigadierArgument<'a> for GreedyString<'a> {
    fn parser(_data: Option<&Self::Data>) -> Parser<'a> {
        Parser::String(StringArg::GreedyPhrase)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RelativeValue<T> {
    Relative(T),
    Absolute(T),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum RelativeValueError<E> {
    MixedPos,
    Value(E),
}

impl<E: ParsingBuild<ParsingError>> ParsingBuild<ParsingError> for RelativeValueError<E> {
    fn build(self) -> ParsingError {
        match self {
            Self::MixedPos => ParsingError::translate(ARGUMENT_POS_MIXED, vec![]),
            Self::Value(err) => err.build(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RelativeValueSuggestion<'a>(&'a str);

impl<'a> ParsingBuild<ParsingSuggestions<'a>> for RelativeValueSuggestion<'a> {
    fn build(self) -> ParsingSuggestions<'a> {
        ParsingSuggestions::Owned(if self.0.starts_with("~") {
            vec![
                self.0.into(),
                unsafe { self.0.get_unchecked('~'.len_utf8()..) }.into(),
            ]
        } else {
            vec![
                {
                    let mut str = String::new();
                    str.push('~');
                    str.push_str(self.0);
                    str
                }
                .into(),
                self.0.into(),
            ]
        })
    }
}

impl<'a, T: Parsable<'a>> Parsable<'a> for RelativeValue<T> {
    type Data = T::Data;

    type Error = RelativeValueError<T::Error>;

    type Suggestions = RelativeValueSuggestion<'a>;

    fn parse(
        data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        purpose: ParsingPurpose,
    ) -> ParsingResult<'a, Self> {
        let begin = reader.cursor();

        let relative = match reader.peek_char() {
            Some('^') => {
                reader.next_char();
                Err(RelativeValueError::MixedPos)
            }
            Some('~') => {
                reader.next_char();
                Ok(true)
            }
            _ => Ok(false),
        };

        let t_result = T::parse(
            data,
            reader,
            match relative {
                Err(_) => ParsingPurpose::Suggestion,
                Ok(_) => purpose,
            },
        );

        let result = relative
            .map_err(|err| (begin..reader.cursor(), err))
            .and_then(|relative| {
                t_result
                    .result
                    .map(|v| {
                        v.map(|v| {
                            if relative {
                                Self::Relative(v)
                            } else {
                                Self::Absolute(v)
                            }
                        })
                    })
                    .map_err(|(pos, err)| (pos, RelativeValueError::Value(err)))
            });

        ParsingResult {
            suggestions: Some((
                begin..reader.cursor(),
                RelativeValueSuggestion(unsafe {
                    reader
                        .str
                        .get_unchecked(begin + if result.is_err() { 1 } else { 0 }..reader.cursor())
                }),
            )),
            result,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Angle(pub RelativeValue<f32>);

parsing_error!(AngleError {
    MixedPos = ARGUMENT_POS_MIXED,
    Invalid = ARGUMENT_ANGLE_INVALID,
});

impl<'a> Parsable<'a> for Angle {
    type Data = ();

    type Error = AngleError;

    type Suggestions = RelativeValueSuggestion<'a>;

    fn parse(
        _data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        purpose: ParsingPurpose,
    ) -> ParsingResult<'a, Self> {
        let result = RelativeValue::parse(Some(&(Some(-180.0), Some(180.0))), reader, purpose);
        ParsingResult {
            suggestions: result.suggestions,
            result: result
                .result
                .map(|v| v.map(|v| Self(v)))
                .map_err(|(pos, err)| {
                    (
                        pos,
                        match err {
                            RelativeValueError::MixedPos => AngleError::MixedPos,
                            RelativeValueError::Value(_) => AngleError::Invalid,
                        },
                    )
                }),
        }
    }
}

impl<'a> BrigadierArgument<'a> for Angle {
    fn parser(_data: Option<&Self::Data>) -> Parser<'a> {
        Parser::Angle
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ArrayError<E> {
    Value(E),
    Separator,
}

impl<E: ParsingBuild<ParsingError>> ParsingBuild<ParsingError> for ArrayError<E> {
    fn build(self) -> ParsingError {
        match self {
            Self::Value(e) => e.build(),
            Self::Separator => ParsingError::translate(COMMAND_EXPECTED_SEPARATOR, vec![]),
        }
    }
}

impl<E> From<E> for ArrayError<E> {
    fn from(value: E) -> Self {
        Self::Value(value)
    }
}

impl<'a, const C: usize, T: Parsable<'a>> Parsable<'a> for [T; C] {
    type Data = [Option<T::Data>; C];

    type Error = ArrayError<T::Error>;

    type Suggestions = T::Suggestions;

    fn parse(
        data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        purpose: ParsingPurpose,
    ) -> ParsingResult<'a, Self> {
        // I hope that compiler will optimize this option thing.
        let mut values: Option<[MaybeUninit<T>; C]> = match purpose {
            // SAFETY: uninited MaybeUninit is not UB
            ParsingPurpose::Reading => Some(unsafe { MaybeUninit::uninit().assume_init() }),
            ParsingPurpose::Suggestion => None,
        };

        macro_rules! t_write {
            ($i:expr) => {{
                let t_result = T::parse(data.and_then(|v| v[$i].as_ref()), reader, purpose);
                let v = match t_result.result {
                    Ok(value) => value,
                    Err((pos, err)) => {
                        return ParsingResult {
                            suggestions: t_result.suggestions,
                            result: Err((pos, err.into())),
                        };
                    }
                };
                if let Some(ref mut values) = values {
                    values[$i].write(v.expect("Purpose is Reading, but the given value is none"));
                }
            }};
        }

        t_write!(0);

        for i in 1..C {
            let begin = reader.cursor();

            if reader.next_char() != Some(' ') {
                return ParsingResult {
                    suggestions: None,
                    result: Err((begin..reader.cursor(), ArrayError::Separator)),
                };
            }

            t_write!(i);
        }

        ParsingResult {
            suggestions: None,
            result: Ok(values.map(|values| unsafe { values.map(|v| v.assume_init()) })),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VectorA<const C: usize, T> {
    Relative([T; C]),
    Absolute([RelativeValue<T>; C]),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VectorASuggestions<'a> {
    Everything,
    Caret(&'a str),
    Absolute(&'a str),
}

const EVERYTHING_SUGGESTIONS: &'static [Suggestion<'static>] =
    &[Suggestion::new_str("~"), Suggestion::new_str("^")];

impl<'a> ParsingBuild<ParsingSuggestions<'a>> for VectorASuggestions<'a> {
    fn build(self) -> ParsingSuggestions<'a> {
        match self {
            Self::Everything => ParsingSuggestions::Borrowed(EVERYTHING_SUGGESTIONS),
            Self::Caret(given) => ParsingSuggestions::Owned(vec![{
                let mut str = String::new();
                str.push('^');
                str.push_str(given);
                str.into()
            }]),
            Self::Absolute(given) => RelativeValueSuggestion(given).build(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum VectorAError<E> {
    MixedPos,
    Value(E),
    Separator,
}

impl<E: ParsingBuild<ParsingError>> ParsingBuild<ParsingError> for VectorAError<E> {
    fn build(self) -> ParsingError {
        match self {
            Self::MixedPos => RelativeValueError::<E>::MixedPos.build(),
            Self::Value(value) => RelativeValueError::Value(value).build(),
            Self::Separator => ParsingError::translate(COMMAND_EXPECTED_SEPARATOR, vec![]),
        }
    }
}

impl<E> From<RelativeValueError<E>> for VectorAError<E> {
    fn from(value: RelativeValueError<E>) -> Self {
        match value {
            RelativeValueError::MixedPos => Self::MixedPos,
            RelativeValueError::Value(e) => Self::Value(e),
        }
    }
}

impl<E> From<E> for VectorAError<E> {
    fn from(value: E) -> Self {
        Self::Value(value)
    }
}

impl<'a, const C: usize, T: Parsable<'a> + Sized> Parsable<'a> for VectorA<C, T> {
    type Data = [Option<T::Data>; C];

    type Error = VectorAError<T::Error>;

    type Suggestions = VectorASuggestions<'a>;

    fn parse(
        data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        purpose: ParsingPurpose,
    ) -> ParsingResult<'a, Self> {
        if reader.peek_char() == Some('^') {
            #[repr(transparent)]
            struct CaretValue<T>(T);

            impl<'a, T: Parsable<'a> + Sized> Parsable<'a> for CaretValue<T> {
                type Data = T::Data;

                type Error = <VectorA<0, T> as Parsable<'a>>::Error;

                type Suggestions = <VectorA<0, T> as Parsable<'a>>::Suggestions;

                fn parse(
                    data: Option<&Self::Data>,
                    reader: &mut StrReader<'a>,
                    purpose: ParsingPurpose,
                ) -> ParsingResult<'a, Self> {
                    let begin = reader.cursor();

                    if reader.next_char() != Some('^') {
                        return ParsingResult {
                            suggestions: Some((
                                begin..reader.cursor(),
                                VectorASuggestions::Caret(""),
                            )),
                            result: Err((begin..reader.cursor(), VectorAError::MixedPos)),
                        };
                    }

                    let t_result = T::parse(data, reader, purpose);

                    ParsingResult {
                        suggestions: None,
                        result: t_result
                            .result
                            .map(|v| v.map(|v| CaretValue(v)))
                            .map_err(|(pos, err)| (pos, err.into())),
                    }
                }
            }

            let result = <[CaretValue<T>; C]>::parse(data, reader, purpose);

            ParsingResult {
                suggestions: result.suggestions,
                result: result
                    .result
                    .map(|v| v.map(|v| Self::Relative(v.map(|v| v.0))))
                    .map_err(|(pos, err)| {
                        (
                            pos,
                            match err {
                                ArrayError::Separator => VectorAError::Separator,
                                ArrayError::Value(e) => e,
                            },
                        )
                    }),
            }
        } else {
            let result = <[RelativeValue<T>; C]>::parse(data, reader, purpose);

            ParsingResult {
                suggestions: result
                    .suggestions
                    .map(|(pos, s)| (pos, VectorASuggestions::Absolute(s.0))),
                result: result.result.map(|v| v.map(|v| Self::Absolute(v))).map_err(
                    |(pos, err)| {
                        (
                            pos,
                            match err {
                                ArrayError::Separator => VectorAError::Separator,
                                ArrayError::Value(e) => e.into(),
                            },
                        )
                    },
                ),
            }
        }
    }
}

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
    ) -> ParsingResult<'a, Self> {
        let mut begin = reader.cursor();

        let kind = if reader.peek_char() == Some('#') {
            reader.next_char();
            BlockPredicateKind::Tag(reader.read_ident_str())
        } else {
            let kind_str = reader.read_ident_str();
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

        let kind_str = unsafe { reader.str.get_unchecked(begin..reader.cursor()) };

        let mut states = vec![];
        if reader.skip_char('[') {
            if !reader.skip_char(']') {
                loop {
                    begin = reader.cursor();
                    let prop_name_str = reader.read_unquoted_str();
                    let prop_name = match PropName::from_str(prop_name_str) {
                        Some(n) => n,
                        None => {
                            return ParsingResult {
                                suggestions: Some((
                                    begin..reader.cursor(),
                                    BlockSuggestions::PropName,
                                )),
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
                    begin = reader.cursor();
                    if !reader.skip_char('=') {
                        return ParsingResult {
                            suggestions: Some((
                                begin..reader.cursor(),
                                BlockSuggestions::EqualSign,
                            )),
                            result: Err((
                                begin..reader.cursor(),
                                BlockError::PropNoValue {
                                    kind: kind_str,
                                    name: prop_name_str,
                                },
                            )),
                        };
                    }
                    reader.skip_char(' ');
                    begin = reader.cursor();
                    let prop_value_str = reader.read_unquoted_str();
                    let prop_value = match PropValue::from_str(prop_value_str) {
                        Some(v) => v,
                        None => {
                            return ParsingResult {
                                suggestions: Some((
                                    begin..reader.cursor(),
                                    BlockSuggestions::PropValue,
                                )),
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

                    if let ParsingPurpose::Reading = purpose {
                        states.push((prop_name, prop_value));
                    }

                    reader.skip_char(' ');

                    begin = reader.cursor();

                    match reader.next_char() {
                        Some(',') => {}
                        Some(']') => {
                            break;
                        }
                        _ => {
                            return ParsingResult {
                                suggestions: Some((
                                    begin..reader.cursor(),
                                    BlockSuggestions::StateEnd,
                                )),
                                result: Err((begin..reader.cursor(), BlockError::PropUnclosed)),
                            }
                        }
                    };
                }
            }
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bool_test() {
        let mut reader = StrReader::new("true false bad");

        assert_eq!(
            bool::parse(None, &mut reader, ParsingPurpose::Reading),
            ParsingResult {
                suggestions: Some((0..4, BoolSuggestions)),
                result: Ok(Some(true))
            }
        );

        assert_eq!(reader.next_char(), Some(' '));

        assert_eq!(
            bool::parse(None, &mut reader, ParsingPurpose::Reading),
            ParsingResult {
                suggestions: Some((5..10, BoolSuggestions)),
                result: Ok(Some(false)),
            }
        );

        assert_eq!(reader.next_char(), Some(' '));

        assert_eq!(
            bool::parse(None, &mut reader, ParsingPurpose::Reading),
            ParsingResult {
                suggestions: Some((11..14, BoolSuggestions)),
                result: Err((11..14, BoolParsingError::Invalid("bad"))),
            }
        );
    }

    #[test]
    fn num_test() {
        let mut reader = StrReader::new("10 30 40.0 50.0");

        assert_eq!(
            i32::parse(None, &mut reader, ParsingPurpose::Reading),
            ParsingResult {
                suggestions: None,
                result: Ok(Some(10))
            }
        );

        assert_eq!(reader.next_char(), Some(' '));

        assert_eq!(
            i32::parse(
                Some(&(Some(40), None)),
                &mut reader,
                ParsingPurpose::Reading
            ),
            ParsingResult {
                suggestions: None,
                result: Err((3..5, I32Error::TooLow("30", 40)))
            }
        );

        unsafe { reader.set_cursor(5) };

        assert_eq!(reader.next_char(), Some(' '));

        assert_eq!(
            f32::parse(None, &mut reader, ParsingPurpose::Reading),
            ParsingResult {
                suggestions: None,
                result: Ok(Some(40.0))
            }
        );

        assert_eq!(reader.next_char(), Some(' '));

        assert_eq!(
            f32::parse(
                Some(&(None, Some(40.0))),
                &mut reader,
                ParsingPurpose::Reading
            ),
            ParsingResult {
                suggestions: None,
                result: Err((11..15, F32Error::TooBig("50.0", 40.0)))
            }
        );
    }

    #[test]
    fn string_test() {
        let mut reader = StrReader::new(r#"aba "aba aba" "aba"#);

        assert_eq!(
            SingleWordString::parse(None, &mut reader, ParsingPurpose::Reading),
            ParsingResult {
                suggestions: None,
                result: Ok(Some(SingleWordString("aba")))
            }
        );

        assert_eq!(reader.next_char(), Some(' '));

        assert_eq!(
            QuotableString::parse(None, &mut reader, ParsingPurpose::Reading),
            ParsingResult {
                suggestions: None,
                result: Ok(Some(QuotableString(Cow::Owned("aba aba".into()))))
            }
        );

        assert_eq!(reader.next_char(), Some(' '));

        assert_eq!(
            QuotableString::parse(None, &mut reader, ParsingPurpose::Reading),
            ParsingResult {
                suggestions: None,
                result: Err((14..18, UnclosedQuoteError)),
            }
        );
    }

    #[test]
    fn relative_value_test() {
        assert_eq!(
            RelativeValue::parse(None, &mut StrReader::new("~32"), ParsingPurpose::Reading),
            ParsingResult {
                suggestions: Some((0..3, RelativeValueSuggestion("~32"))),
                result: Ok(Some(RelativeValue::Relative(32))),
            }
        );

        assert_eq!(
            RelativeValue::parse(None, &mut StrReader::new("42"), ParsingPurpose::Reading),
            ParsingResult {
                suggestions: Some((0..2, RelativeValueSuggestion("42"))),
                result: Ok(Some(RelativeValue::Absolute(42))),
            }
        );
    }

    #[test]
    fn vector_test() {
        assert_eq!(
            VectorA::parse(
                None,
                &mut StrReader::new("^32 ^32 ^90"),
                ParsingPurpose::Reading
            ),
            ParsingResult {
                suggestions: None,
                result: Ok(Some(VectorA::Relative([32, 32, 90])))
            }
        );

        assert_eq!(
            VectorA::parse(
                None,
                &mut StrReader::new("32 ~32 90"),
                ParsingPurpose::Reading
            ),
            ParsingResult {
                suggestions: None,
                result: Ok(Some(VectorA::Absolute([
                    RelativeValue::Absolute(32),
                    RelativeValue::Relative(32),
                    RelativeValue::Absolute(90)
                ])))
            }
        );

        assert_eq!(
            VectorA::<3, i32>::parse(
                None,
                &mut StrReader::new("32 ^32 90"),
                ParsingPurpose::Reading
            ),
            ParsingResult {
                suggestions: Some((3..6, VectorASuggestions::Absolute("32"))),
                result: Err((3..6, VectorAError::MixedPos)),
            }
        );
    }

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
        )
    }
}
