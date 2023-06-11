use std::borrow::Cow;
use std::num::NonZeroI32;

use valence_core::game_mode::GameMode;
use valence_core::translation_key::{
    ARGUMENT_ENTITY_OPTIONS_INAPPLICABLE, ARGUMENT_ENTITY_OPTIONS_LIMIT_TOOSMALL,
    ARGUMENT_ENTITY_OPTIONS_SORT_IRREVERSIBLE, ARGUMENT_ENTITY_OPTIONS_UNKNOWN,
    ARGUMENT_ENTITY_OPTIONS_UNTERMINATED, ARGUMENT_ENTITY_OPTIONS_VALUELESS,
    ARGUMENT_ENTITY_SELECTOR_MISSING, ARGUMENT_ENTITY_SELECTOR_UNKNOWN, PARSING_EXPECTED,
};
use valence_nbt::Compound;

use crate::cenum::{CEnumError, CEnumSuggestions};
use crate::parse_util::parse_array_like;
use crate::parser::{
    NoParsingBuild, Parse, ParsingBuild, ParsingError, ParsingPurpose, ParsingResult,
    ParsingSuggestions, Suggestion,
};
use crate::primitive::{BoolParsingError, BoolSuggestions, F64Error, I32Error, SingleWordString};
use crate::range::{InclusiveRange, InclusiveRangeError};
use crate::reader::StrReader;
use crate::resource::ResourceLocation;
use crate::{cenum, p_try, parsing_token};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct EntitySelector<'a> {
    pub self_entity: bool,

    /// `x`
    pub pos_x: Option<f64>,
    /// `y`
    pub pos_y: Option<f64>,
    /// `z`
    pub pos_z: Option<f64>,

    /// `distance`
    pub distance: Option<InclusiveRange<f64>>,

    /// `dx`
    pub volume_x: Option<f64>,
    /// `dy`
    pub volume_y: Option<f64>,
    /// `dz`
    pub volume_z: Option<f64>,

    /// `x_rotation`
    pub x_rotation: Option<InclusiveRange<f64>>,
    /// `y_rotation`
    pub y_rotation: Option<InclusiveRange<f64>>,

    /// `scores`
    pub scores: Vec<EntitySelectorScore<'a>>,
    /// `tags`
    pub tags: Vec<EntitySelectorFlag<SingleWordString<'a>>>,
    /// `teams`
    pub teams: Vec<EntitySelectorFlag<SingleWordString<'a>>>,

    /// `name`
    pub names: Vec<EntitySelectorFlag<SingleWordString<'a>>>,
    /// `type`
    pub types: Vec<EntitySelectorFlag<SingleWordString<'a>>>,
    /// `predicate`
    pub predicates: Vec<EntitySelectorFlag<ResourceLocation<'a>>>,

    /// `nbt`
    pub nbt: Option<Compound>,

    /// `level`
    pub level: Option<InclusiveRange<i32>>,
    /// `gamemode`
    pub gamemodes: [Option<bool>; 4],
    /// `advancements`
    pub advancements: Vec<EntitySelectorAdvancement<'a>>,

    /// `limit`
    pub limit: Option<EntitySelectorLimit>,

    /// `sort`
    pub sort: Option<EntitySelectorSort>,
}

impl<'a> EntitySelector<'a> {
    pub fn limit_to_one(&mut self) {
        // SAFETY: 1 is not 0
        self.limit = Some(EntitySelectorLimit(unsafe { NonZeroI32::new_unchecked(1) }));
    }

    pub fn limit_to_player(&mut self) {
        self.types.push(EntitySelectorFlag {
            flag: true,
            value: SingleWordString("player"),
        });
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum EntitySelectorSort {
    #[default]
    Arbitrary,
    Nearest,
    Furthest,
    Random,
}

cenum!(EntitySelectorSort; ARGUMENT_ENTITY_OPTIONS_SORT_IRREVERSIBLE => {
    Arbitrary,
    Nearest,
    Furthest,
    Random,
});

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EntitySelectorError<'a> {
    ExpectedEnd,
    ExpectedValue(&'a str),
    UnknownOption(&'a str),
    UnknownSelectorType(char),
    MissingSelectorType,
    InapplicableOption(&'a str),
    F64Error(F64Error<'a>),
    I32Error(I32Error<'a>),
    BoolError(BoolParsingError<'a>),
    F64InclusiveRange(InclusiveRangeError<F64Error<'a>>),
    I32InclusiveRange(InclusiveRangeError<I32Error<'a>>),
    OptionEnterExpected,
    OptionEndExpected,
    EqSignExpected,
    LimitTooSmall,
    Sort(CEnumError<'a, EntitySelectorSort>),
    GameMode(CEnumError<'a, GameMode>),
}

impl<'a> From<F64Error<'a>> for EntitySelectorError<'a> {
    fn from(value: F64Error<'a>) -> Self {
        Self::F64Error(value)
    }
}

impl<'a> From<I32Error<'a>> for EntitySelectorError<'a> {
    fn from(value: I32Error<'a>) -> Self {
        Self::I32Error(value)
    }
}

impl<'a> From<BoolParsingError<'a>> for EntitySelectorError<'a> {
    fn from(value: BoolParsingError<'a>) -> Self {
        Self::BoolError(value)
    }
}

impl<'a> From<InclusiveRangeError<F64Error<'a>>> for EntitySelectorError<'a> {
    fn from(value: InclusiveRangeError<F64Error<'a>>) -> Self {
        Self::F64InclusiveRange(value)
    }
}

impl<'a> From<InclusiveRangeError<I32Error<'a>>> for EntitySelectorError<'a> {
    fn from(value: InclusiveRangeError<I32Error<'a>>) -> Self {
        Self::I32InclusiveRange(value)
    }
}

impl<'a> From<CEnumError<'a, EntitySelectorSort>> for EntitySelectorError<'a> {
    fn from(value: CEnumError<'a, EntitySelectorSort>) -> Self {
        Self::Sort(value)
    }
}

impl<'a> From<CEnumError<'a, GameMode>> for EntitySelectorError<'a> {
    fn from(value: CEnumError<'a, GameMode>) -> Self {
        Self::GameMode(value)
    }
}

impl<'a> From<NoParsingBuild> for EntitySelectorError<'a> {
    fn from(_value: NoParsingBuild) -> Self {
        unreachable!();
    }
}

impl<'a> ParsingBuild<ParsingError> for EntitySelectorError<'a> {
    fn build(self) -> ParsingError {
        match self {
            Self::ExpectedEnd => {
                ParsingError::translate(ARGUMENT_ENTITY_OPTIONS_UNTERMINATED, vec![])
            }
            Self::ExpectedValue(name) => ParsingError::translate(
                ARGUMENT_ENTITY_OPTIONS_VALUELESS,
                vec![name.to_string().into()],
            ),
            Self::UnknownOption(name) => ParsingError::translate(
                ARGUMENT_ENTITY_OPTIONS_UNKNOWN,
                vec![name.to_string().into()],
            ),
            Self::UnknownSelectorType(name) => ParsingError::translate(
                ARGUMENT_ENTITY_SELECTOR_UNKNOWN,
                vec![{
                    let mut str = '@'.to_string();
                    str.push(name);
                    str
                }
                .into()],
            ),
            Self::MissingSelectorType => {
                ParsingError::translate(ARGUMENT_ENTITY_SELECTOR_MISSING, vec![])
            }
            Self::InapplicableOption(name) => ParsingError::translate(
                ARGUMENT_ENTITY_OPTIONS_INAPPLICABLE,
                vec![name.to_string().into()],
            ),
            Self::F64Error(err) => err.build(),
            Self::I32Error(err) => err.build(),
            Self::BoolError(err) => err.build(),
            Self::F64InclusiveRange(err) => err.build(),
            Self::I32InclusiveRange(err) => err.build(),
            Self::OptionEnterExpected => {
                ParsingError::translate(PARSING_EXPECTED, vec!['{'.to_string().into()])
            }
            Self::OptionEndExpected => {
                ParsingError::translate(PARSING_EXPECTED, vec!['}'.to_string().into()])
            }
            Self::EqSignExpected => {
                ParsingError::translate(PARSING_EXPECTED, vec!['='.to_string().into()])
            }
            Self::LimitTooSmall => {
                ParsingError::translate(ARGUMENT_ENTITY_OPTIONS_LIMIT_TOOSMALL, vec![])
            }
            Self::Sort(err) => err.build(),
            Self::GameMode(err) => err.build(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EntitySelectorSuggestions {
    Next,
    Name,
    Selector,
    EqSign,
    OptionEnter,
    OptionEnd,
    Bool(BoolSuggestions),
    Sort(CEnumSuggestions<EntitySelectorSort>),
    GameMode(EntitySelectorFlagSuggestions<CEnumSuggestions<GameMode>>),
}

impl From<NoParsingBuild> for EntitySelectorSuggestions {
    fn from(_value: NoParsingBuild) -> Self {
        unreachable!()
    }
}

impl From<EntitySelectorFlagSuggestions<NoParsingBuild>> for EntitySelectorSuggestions {
    fn from(_value: EntitySelectorFlagSuggestions<NoParsingBuild>) -> Self {
        unreachable!()
    }
}

impl From<BoolSuggestions> for EntitySelectorSuggestions {
    fn from(value: BoolSuggestions) -> Self {
        Self::Bool(value)
    }
}

impl From<CEnumSuggestions<EntitySelectorSort>> for EntitySelectorSuggestions {
    fn from(value: CEnumSuggestions<EntitySelectorSort>) -> Self {
        Self::Sort(value)
    }
}

impl From<EntitySelectorFlagSuggestions<CEnumSuggestions<GameMode>>> for EntitySelectorSuggestions {
    fn from(value: EntitySelectorFlagSuggestions<CEnumSuggestions<GameMode>>) -> Self {
        Self::GameMode(value)
    }
}

impl<'a> ParsingBuild<ParsingSuggestions<'a>> for EntitySelectorSuggestions {
    fn build(self) -> ParsingSuggestions<'a> {
        const NEXT: &[Suggestion<'static>] = &[Suggestion::new_str(","), Suggestion::new_str("]")];
        const PROP_NAMES: &[Suggestion<'static>] = &[];
        const SELECTORS: &[Suggestion<'static>] = &[
            Suggestion::new_str("p"),
            Suggestion::new_str("r"),
            Suggestion::new_str("a"),
            Suggestion::new_str("s"),
            Suggestion::new_str("e"),
        ];
        const EQ_SIGN: &[Suggestion<'static>] = &[Suggestion::new_str("=")];
        const OPTION_ENTER: &[Suggestion<'static>] = &[Suggestion::new_str("{")];
        const OPTION_END: &[Suggestion<'static>] = &[Suggestion::new_str("}")];
        match self {
            Self::Next => ParsingSuggestions::Borrowed(NEXT),
            Self::Name => ParsingSuggestions::Borrowed(PROP_NAMES),
            Self::Selector => ParsingSuggestions::Borrowed(SELECTORS),
            Self::EqSign => ParsingSuggestions::Borrowed(EQ_SIGN),
            Self::OptionEnter => ParsingSuggestions::Borrowed(OPTION_ENTER),
            Self::OptionEnd => ParsingSuggestions::Borrowed(OPTION_END),
            Self::Bool(s) => s.build(),
            Self::Sort(s) => s.build(),
            Self::GameMode(s) => s.build(),
        }
    }
}

impl<'a> Parse<'a> for EntitySelector<'a> {
    type Data = ();

    type Error = EntitySelectorError<'a>;

    type Suggestions = EntitySelectorSuggestions;

    fn parse(
        _data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        purpose: ParsingPurpose,
    ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
        let mut result = EntitySelector::default();

        let begin = reader.cursor();

        match reader.next_char() {
            Some('p') => {
                result.limit_to_one();
                result.limit_to_player();
                result.sort = Some(EntitySelectorSort::Nearest);
            }
            Some('r') => {
                result.limit_to_one();
                result.limit_to_player();
                result.sort = Some(EntitySelectorSort::Random);
            }
            Some('a') => {
                result.limit_to_player();
            }
            Some('e') => {}
            Some('s') => {
                result.self_entity = true;
            }
            Some(ch) => {
                return ParsingResult {
                    suggestions: Some((
                        begin..reader.cursor(),
                        EntitySelectorSuggestions::Selector,
                    )),
                    result: Err((
                        begin..reader.cursor(),
                        EntitySelectorError::UnknownSelectorType(ch),
                    )),
                }
            }
            None => {
                return ParsingResult {
                    suggestions: Some((begin..begin, EntitySelectorSuggestions::Selector)),
                    result: Err((begin..begin, EntitySelectorError::MissingSelectorType)),
                }
            }
        }

        if reader.skip_char('[') {
            p_try!(parse_array_like(
                reader,
                (
                    EntitySelectorSuggestions::Next,
                    EntitySelectorError::ExpectedEnd
                ),
                ']',
                |reader| {
                    let begin = reader.cursor();
                    let name = reader.read_unquoted_str();
                    let name_end = reader.cursor();
                    reader.skip_char(' ');

                    parsing_token!(
                        reader,
                        '=',
                        EntitySelectorError::ExpectedValue(name),
                        EntitySelectorSuggestions::Name,
                    );

                    reader.skip_char(' ');

                    macro_rules! inapplicable {
                        () => {
                            return ParsingResult {
                                suggestions: None,
                                result: Err((
                                    begin..name_end,
                                    EntitySelectorError::InapplicableOption(name),
                                )),
                            };
                        };
                    }

                    macro_rules! parse_single_option {
                        ($ty:ty, $field:ident) => {{
                            if result.$field.is_some() {
                                inapplicable!();
                            }

                            result.$field.replace(
                                p_try!(<$ty>::parse(None, reader, purpose))
                                    .1
                                    .unwrap_or_default(),
                            );
                        }};
                    }

                    macro_rules! parse_multi_option {
                        ($ty:ty, $field:ident) => {{
                            let (_, o) = p_try!(<$ty>::parse(None, reader, purpose));
                            if let ParsingPurpose::Reading = purpose {
                                result.$field.push(o.expect("ParsingPurpose is reading"));
                            }
                        }};
                    }

                    macro_rules! parse_array_like_option {
                        ($ty:ty, $field:ident) => {{
                            let _begin = reader.cursor();

                            parsing_token!(
                                reader,
                                '{',
                                EntitySelectorError::OptionEnterExpected,
                                EntitySelectorSuggestions::OptionEnter,
                            );

                            while !reader.skip_char('}') {
                                reader.skip_char(' ');
                                let (_, o) = p_try!(<$ty>::parse(None, reader, purpose));
                                if let ParsingPurpose::Reading = purpose {
                                    result.$field.push(o.expect("ParsingPurpose is reading"));
                                }
                                reader.skip_char(' ');
                                reader.skip_char(',');
                            }
                        }};
                    }

                    match name {
                        "x" => parse_single_option!(f64, pos_x),
                        "y" => parse_single_option!(f64, pos_y),
                        "z" => parse_single_option!(f64, pos_z),
                        "distance" => parse_single_option!(InclusiveRange<f64>, distance),
                        "dx" => parse_single_option!(f64, volume_x),
                        "dy" => parse_single_option!(f64, volume_y),
                        "dz" => parse_single_option!(f64, volume_z),
                        "x_rotation" => parse_single_option!(InclusiveRange<f64>, x_rotation),
                        "y_rotation" => parse_single_option!(InclusiveRange<f64>, y_rotation),
                        "scores" => parse_array_like_option!(EntitySelectorScore, scores),
                        "tag" => parse_multi_option!(EntitySelectorFlag<SingleWordString>, tags),
                        "team" => parse_multi_option!(EntitySelectorFlag<SingleWordString>, teams),
                        "name" => parse_multi_option!(EntitySelectorFlag<SingleWordString>, names),
                        "type" => parse_multi_option!(EntitySelectorFlag<SingleWordString>, types),
                        "predicate" => {
                            parse_multi_option!(EntitySelectorFlag<ResourceLocation>, predicates)
                        }
                        "level" => parse_single_option!(InclusiveRange<i32>, level),
                        "gamemode" => {
                            let EntitySelectorFlag { flag, value } =
                                p_try!(EntitySelectorFlag::<GameMode>::parse(
                                    None,
                                    reader,
                                    ParsingPurpose::Reading
                                ))
                                .1
                                .expect("ParsingPurpose is reading");
                            let i = value.to_index();
                            if result.gamemodes[i].is_some() {
                                inapplicable!();
                            }
                            result.gamemodes[i] = Some(flag);
                        }
                        "advancements" => {
                            parse_array_like_option!(EntitySelectorAdvancement, advancements)
                        }
                        "limit" => parse_single_option!(EntitySelectorLimit, limit),
                        "sort" => parse_single_option!(EntitySelectorSort, sort),
                        _ => {
                            return ParsingResult {
                                suggestions: Some((
                                    begin..name_end,
                                    EntitySelectorSuggestions::Name,
                                )),
                                result: Err((
                                    begin..name_end,
                                    EntitySelectorError::UnknownOption(name),
                                )),
                            }
                        }
                    }

                    ParsingResult::ok()
                }
            ));
        }

        ParsingResult {
            suggestions: None,
            result: Ok(match purpose {
                ParsingPurpose::Reading => Some(result),
                ParsingPurpose::Suggestion => None,
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct EntitySelectorFlag<T> {
    pub flag: bool,
    pub value: T,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntitySelectorFlagSuggestions<S>(S);

impl<'a, S: ParsingBuild<ParsingSuggestions<'a>>> ParsingBuild<ParsingSuggestions<'a>>
    for EntitySelectorFlagSuggestions<S>
{
    fn build(self) -> ParsingSuggestions<'a> {
        let built = self.0.build();
        let mut result = Vec::with_capacity(built.len() * 2);
        match built {
            ParsingSuggestions::Owned(arr) => {
                for sug in arr {
                    result.push(Suggestion {
                        message: {
                            let mut res = "!".to_string();
                            res.push_str(&sug.message);
                            Cow::Owned(res)
                        },
                        tooltip: sug.tooltip.clone(),
                    });
                    result.push(sug);
                }
            }
            ParsingSuggestions::Borrowed(arr) => {
                for sug in arr {
                    result.push(Suggestion {
                        message: {
                            let mut res = "!".to_string();
                            res.push_str(&sug.message);
                            Cow::Owned(res)
                        },
                        tooltip: sug.tooltip.clone(),
                    });
                    result.push(sug.clone());
                }
            }
        }
        ParsingSuggestions::Owned(result)
    }
}

impl<'a, T: Parse<'a>> Parse<'a> for EntitySelectorFlag<T> {
    type Data = T::Data;

    type Error = T::Error;

    type Suggestions = EntitySelectorFlagSuggestions<T::Suggestions>;

    fn parse(
        data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        purpose: ParsingPurpose,
    ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
        let flag = !reader.skip_char('!');

        T::parse(data, reader, purpose)
            .map_suggestion(EntitySelectorFlagSuggestions)
            .map_ok(|value| Self { flag, value })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntitySelectorScore<'a> {
    pub name: &'a str,
    pub value: InclusiveRange<i32>,
}

impl<'a> Parse<'a> for EntitySelectorScore<'a> {
    type Data = ();

    type Error = EntitySelectorError<'a>;

    type Suggestions = EntitySelectorSuggestions;

    fn parse(
        _data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        purpose: ParsingPurpose,
    ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
        let name = reader.read_unquoted_str();

        reader.skip_char(' ');
        let _begin = reader.cursor();
        parsing_token!(
            reader,
            '=',
            EntitySelectorError::EqSignExpected,
            EntitySelectorSuggestions::EqSign
        );
        reader.skip_char(' ');

        ParsingResult {
            suggestions: None,
            result: Ok(p_try!(InclusiveRange::<i32>::parse(None, reader, purpose))
                .1
                .map(|value| Self { name, value })),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntitySelectorAdvancement<'a> {
    pub advancement: &'a str,
    pub criteria: Option<&'a str>,
    pub completed: bool,
}

impl<'a> Parse<'a> for EntitySelectorAdvancement<'a> {
    type Data = ();

    type Error = EntitySelectorError<'a>;

    type Suggestions = EntitySelectorSuggestions;

    fn parse(
        _data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        purpose: ParsingPurpose,
    ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
        let advancement = reader.read_resource_location_str();

        let _begin = reader.cursor();
        parsing_token!(
            reader,
            '=',
            EntitySelectorError::EqSignExpected,
            EntitySelectorSuggestions::EqSign
        );

        let (criteria, value) = if reader.skip_char('{') {
            let criteria = reader.read_resource_location_str();
            let _begin = reader.cursor();
            parsing_token!(
                reader,
                '=',
                EntitySelectorError::EqSignExpected,
                EntitySelectorSuggestions::EqSign
            );
            let value = p_try!(bool::parse(None, reader, purpose)).1;
            parsing_token!(
                reader,
                '}',
                EntitySelectorError::OptionEndExpected,
                EntitySelectorSuggestions::OptionEnd,
            );

            (Some(criteria), value)
        } else {
            (None, p_try!(bool::parse(None, reader, purpose)).1)
        };

        ParsingResult {
            suggestions: None,
            result: Ok(match purpose {
                ParsingPurpose::Reading => Some(Self {
                    advancement,
                    criteria,
                    completed: value.expect("ParsingPurpose is reading"),
                }),
                ParsingPurpose::Suggestion => None,
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntitySelectorLimit(NonZeroI32);

impl<'a> Parse<'a> for EntitySelectorLimit {
    type Data = ();

    type Error = EntitySelectorError<'a>;

    type Suggestions = EntitySelectorSuggestions;

    fn parse(
        _data: Option<&Self::Data>,
        reader: &mut StrReader<'a>,
        _purpose: ParsingPurpose,
    ) -> ParsingResult<Self, Self::Suggestions, Self::Error> {
        let begin = reader.cursor();
        let num = p_try!(i32::parse(None, reader, ParsingPurpose::Reading))
            .1
            .expect("ParsingPurpose is reading");

        ParsingResult {
            suggestions: None,
            result: if num < 1 {
                Err((begin..reader.cursor(), EntitySelectorError::LimitTooSmall))
            } else {
                // SAFETY: num is more or equals to 1
                Ok(Some(Self(unsafe { NonZeroI32::new_unchecked(num) })))
            },
        }
    }
}

impl Default for EntitySelectorLimit {
    fn default() -> Self {
        Self(NonZeroI32::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entity_selector_parse_test() {
        assert_eq!(
            EntitySelector::parse(
                None,
                &mut StrReader::new("e[name=!Jenya705,type=player]"),
                ParsingPurpose::Reading
            )
            .result
            .unwrap()
            .unwrap(),
            EntitySelector {
                names: vec![EntitySelectorFlag {
                    flag: false,
                    value: SingleWordString("Jenya705"),
                }],
                types: vec![EntitySelectorFlag {
                    flag: true,
                    value: SingleWordString("player"),
                }],
                ..Default::default()
            }
        );
    }
}
