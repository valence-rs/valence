use std::ops::Add;

use thiserror::Error;
use valence_server::protocol::packets::play::command_tree_s2c::{Parser, StringArg};

pub trait CommandArgSet {
    fn from_args(args: Vec<String>) -> Self;
}

pub trait CommandArg:Sized {
    fn arg_from_string(string: String) -> Result<Self, CommandArgParseError> {
        Self::parse_arg(&mut ParseInput::new(string))
    }

    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError>;
    /// what will the client be sent
    fn display() -> Parser;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseInput {
    pub input: String,
    pub cursor: usize,
}

impl ParseInput {
    pub fn new(input: impl Into<String>) -> Self {
        Self { input: input.into(), cursor: 0 }
    }

    pub fn set_cursor(&mut self, cursor: usize) {
        self.cursor = cursor;
    }

    pub fn peek(&self) -> Option<char> {
        self.input.chars().nth(self.cursor)
    }

    pub fn peek_n(&self, n: usize) -> Option<char> {
        self.input.chars().nth(self.cursor + n)
    }

    pub fn is_done(&self) -> bool {
        self.cursor >= self.input.len()
    }

    pub fn advance(&mut self) {
        self.cursor += 1;
    }

    pub fn pop(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.advance();
        }
        c
    }

    pub fn pop_n(&mut self, n: usize) -> Option<String> {
        let s = self.input[self.cursor..self.cursor + n].to_string();
        self.advance_n(n);
        Some(s)
    }

    pub fn pop_to_next(&mut self, c: char) -> Option<String> {
        if let Some(pos) = self.input[self.cursor..].find(c) {
            let s = self.input[self.cursor..self.cursor + pos].to_string();
            self.advance_n(pos);
            Some(s)
        } else {
            None
        }
    }

    pub fn pop_to_next_whitespace_or_end(&mut self) -> Option<String> {
        match self.pop_to_next(' ') {
            Some(s) => Some(s),
            None => {
                let s = self.input[self.cursor..].to_string();
                self.advance_to(self.input.len());
                Some(s)
            }
        }
    }

    pub fn match_next(&mut self, string: &str) -> bool {
        if self.input[self.cursor..].to_lowercase().starts_with(string) {
            self.advance_n(string.len());
            true
        } else {
            false
        }
    }

    pub fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    pub fn advance_n(&mut self, n: usize) {
        self.cursor += n;
    }

    pub fn advance_to(&mut self, to: usize) {
        self.cursor = to;
    }

    pub fn advance_to_next(&mut self, c: char) {
        if let Some(pos) = self.input[self.cursor..].find(c) {
            self.cursor += pos;
        } else {
            self.cursor = self.input.len();
        }
    }

    pub fn advance_to_next_whitespace(&mut self) {
        if let Some(pos) = self.input[self.cursor..].find(char::is_whitespace) {
            self.cursor += pos;
        } else {
            self.cursor = self.input.len();
        }
    }

    pub fn advance_to_next_non_whitespace(&mut self) {
        if let Some(pos) = self.input[self.cursor..].find(|c: char| !c.is_whitespace()) {
            self.cursor += pos;
        } else {
            self.cursor = self.input.len();
        }
    }
}

#[derive(Debug, Error)]
pub enum CommandArgParseError {
    // these should be player facing and not disclose internal information
    #[error("invalid argument, expected {0} got {1}")] // e.g. "integer" number
    InvalidArgument(String, String),
    #[error("invalid argument length")]
    InvalidArgLength,
}

impl CommandArg for bool {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        if input.match_next("true") {
            Ok(true)
        } else if input.match_next("false") {
            Ok(false)
        } else {
            Err(CommandArgParseError::InvalidArgument(
                "bool".to_string(),
                input.input.clone(),
            ))
        }
    }

    fn display() -> Parser {
        Parser::Bool
    }
}

#[test]
fn test_bool() {
    let mut input = ParseInput::new("true".to_string());
    assert!(bool::parse_arg(&mut input).unwrap());
    assert!(input.is_done());

    let mut input = ParseInput::new("false".to_string());
    assert!(!bool::parse_arg(&mut input).unwrap());
    assert!(input.is_done());

    let mut input = ParseInput::new("false ".to_string());
    assert!(!bool::parse_arg(&mut input).unwrap());
    assert!(!input.is_done());

    let mut input = ParseInput::new("falSe trUe".to_string());
    assert!(!bool::parse_arg(&mut input).unwrap());
    assert!(!input.is_done());

    let mut input = ParseInput::new("fAlse true".to_string());
    assert!(!bool::parse_arg(&mut input).unwrap());
    assert!(!input.is_done());

    let mut input = ParseInput::new("false true".to_string());
    assert!(!bool::parse_arg(&mut input).unwrap());
    assert!(!input.is_done());

    let mut input = ParseInput::new("false true".to_string());
    assert!(!bool::parse_arg(&mut input).unwrap());
    assert!(!input.is_done());
}

macro_rules! impl_parser_for_number {
    ($type:ty, $name:expr, $parser:ident) => {
        impl CommandArg for $type {
            fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
                input.skip_whitespace();
                let s = match input.pop_to_next_whitespace_or_end() {
                    Some(s) => s,
                    None => return Err(CommandArgParseError::InvalidArgLength),
                };

                let parsed = s.parse::<$type>();

                parsed
                    .map_err(|_| CommandArgParseError::InvalidArgument($name.to_string(), s))
            }

            fn display() -> Parser {
                Parser::$parser {
                    min: None,
                    max: None,
                }
            }
        }
    };
}

impl_parser_for_number!(f32, "float", Float);
impl_parser_for_number!(f64, "double", Double);
impl_parser_for_number!(i32, "integer", Integer);
impl_parser_for_number!(i64, "long", Long);
impl_parser_for_number!(u32, "unsigned integer", Integer);

#[test]
fn test_number() {
    let mut input = ParseInput::new("1".to_string());
    assert_eq!(1, i32::parse_arg(&mut input).unwrap());
    assert!(input.is_done());

    let mut input = ParseInput::new("1".to_string());
    assert_eq!(1, i64::parse_arg(&mut input).unwrap());
    assert!(input.is_done());

    let mut input = ParseInput::new("1.0".to_string());
    assert_eq!(1.0, f32::parse_arg(&mut input).unwrap());
    assert!(input.is_done());

    let mut input = ParseInput::new("1.0".to_string());
    assert_eq!(1.0, f64::parse_arg(&mut input).unwrap());
    assert!(input.is_done());

    let mut input = ParseInput::new("3.40282347e+38 ".to_string());
    assert_eq!(f32::MAX, f32::parse_arg(&mut input).unwrap());
    assert!(!input.is_done());
}

impl CommandArg for String {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        Ok(match input.pop_to_next_whitespace_or_end() {
            Some(s) => s,
            None => return Err(CommandArgParseError::InvalidArgLength),
        })
    }

    fn display() -> Parser {
        Parser::String(StringArg::SingleWord)
    }
}

#[test]
fn test_string() {
    let mut input = ParseInput::new("hello world".to_string());
    assert_eq!("hello", String::parse_arg(&mut input).unwrap());
    assert!(!input.is_done());
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GreedyString(String);

impl CommandArg for GreedyString {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        Ok(GreedyString(
            match input.pop_n(input.input.len() - input.cursor) {
                Some(s) => s,
                None => return Err(CommandArgParseError::InvalidArgLength),
            },
        ))
    }

    fn display() -> Parser {
        Parser::String(StringArg::GreedyPhrase)
    }
}

#[test]
fn test_greedy_string() {
    let mut input = ParseInput::new("hello world".to_string());
    assert_eq!(
        "hello world",
        GreedyString::parse_arg(&mut input).unwrap().0
    );
    assert!(input.is_done());
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QuotableString(String);

impl CommandArg for QuotableString {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        match input.pop() {
            Some('"') => {
                let mut s = String::new();
                let mut escaped = false;
                while let Some(c) = input.pop() {
                    if escaped {
                        s.push(c);
                        escaped = false;
                    } else if c == '\\' {
                        escaped = true;
                    } else if c == '"' {
                        return Ok(QuotableString(s));
                    } else {
                        s.push(c);
                    }
                }
                Err(CommandArgParseError::InvalidArgLength)
            }
            Some(_) => Ok(QuotableString(String::parse_arg(input)?)),
            None => Err(CommandArgParseError::InvalidArgLength),
        }
    }

    fn display() -> Parser {
        Parser::String(StringArg::QuotablePhrase)
    }
}

#[test]
fn test_quotable_string() {
    let mut input = ParseInput::new("\"hello world\"".to_string());
    assert_eq!(
        "hello world",
        QuotableString::parse_arg(&mut input).unwrap().0
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("\"hello world".to_string());
    assert_eq!(
        "hello world",
        QuotableString::parse_arg(&mut input).unwrap().0
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("hello world\"".to_string());
    assert_eq!("hello", QuotableString::parse_arg(&mut input).unwrap().0);
    assert!(!input.is_done());

    let mut input = ParseInput::new("hello world".to_string());
    assert_eq!("hello", QuotableString::parse_arg(&mut input).unwrap().0);
    assert!(input.is_done());
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntitySelector {
    SimpleSelector(EntitySelectors),
    ComplexSelector(EntitySelectors, String),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum EntitySelectors {
    AllEntities,
    SinglePlayer(String),
    #[default]
    AllPlayers,
    SelfPlayer,
    NearestPlayer,
    RandomPlayer,
}

impl CommandArg for EntitySelector {
    // we want to get either a simple string [`@e`, `@a`, `@p`, `@r`, `<player_name>`] or a full
    // selector: [`@e[<selector>]`, `@a[<selector>]`, `@p[<selector>]`, `@r[<selector>]`]
    // the selectors can have spaces in them, so we need to be careful
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        let mut s = String::new();
        let mut selector = None;
        while let Some(c) = input.pop() {
            match c {
                '@' => {
                    match input.pop() {
                        Some('e') => selector = Some(EntitySelectors::AllEntities),
                        Some('a') => selector = Some(EntitySelectors::AllPlayers),
                        Some('p') => selector = Some(EntitySelectors::NearestPlayer),
                        Some('r') => selector = Some(EntitySelectors::RandomPlayer),
                        Some('s') => selector = Some(EntitySelectors::SelfPlayer),
                        _ => {
                            return Err(CommandArgParseError::InvalidArgument(
                                "entity selector".to_string(),
                                c.to_string(),
                            ))
                        }
                    }
                    if input.peek() != Some('[') {
                        return Ok(EntitySelector::SimpleSelector(selector.unwrap()));
                    }
                }
                '[' => {
                    if selector.is_none() {
                        return Err(CommandArgParseError::InvalidArgument(
                            "entity selector".to_string(),
                            c.to_string(),
                        ));
                    }
                    while let Some(c) = input.pop() {
                        if c == ']' {
                            return Ok(EntitySelector::ComplexSelector(
                                selector.unwrap(),
                                s.trim().to_string(),
                            ));
                        } else {
                            s.push(c);
                        }
                    }
                    return Err(CommandArgParseError::InvalidArgLength);
                }
                _ => {
                    return Err(CommandArgParseError::InvalidArgument(
                        "entity selector".to_string(),
                        c.to_string(),
                    ))
                }
            }
        }
        Err(CommandArgParseError::InvalidArgLength)
    }

    fn display() -> Parser {
        Parser::Entity {
            only_players: false,
            single: false,
        }
    }
}

#[test]
fn test_entity_selector() {
    let mut input = ParseInput::new("@e".to_string());
    assert_eq!(
        EntitySelector::parse_arg(&mut input).unwrap(),
        EntitySelector::SimpleSelector(EntitySelectors::AllEntities)
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("@e[distance=..5]".to_string());
    assert_eq!(
        EntitySelector::parse_arg(&mut input).unwrap(),
        EntitySelector::ComplexSelector(EntitySelectors::AllEntities, "distance=..5".to_string())
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("@s[distance=..5".to_string());
    assert!(EntitySelector::parse_arg(&mut input).is_err());
    assert!(input.is_done());

    let mut input = ParseInput::new("@r[distance=..5] hello".to_string());
    assert_eq!(
        EntitySelector::parse_arg(&mut input).unwrap(),
        EntitySelector::ComplexSelector(EntitySelectors::RandomPlayer, "distance=..5".to_string())
    );
    assert!(!input.is_done());

    let mut input = ParseInput::new("@p[distance=..5]hello".to_string());
    assert_eq!(
        EntitySelector::parse_arg(&mut input).unwrap(),
        EntitySelector::ComplexSelector(EntitySelectors::NearestPlayer, "distance=..5".to_string())
    );
    assert!(!input.is_done());

    let mut input = ParseInput::new("@e[distance=..5] hello world".to_string());
    assert_eq!(
        EntitySelector::parse_arg(&mut input).unwrap(),
        EntitySelector::ComplexSelector(EntitySelectors::AllEntities, "distance=..5".to_string())
    );
    assert!(!input.is_done());

    let mut input = ParseInput::new("@e[distance=..5]hello world".to_string());
    assert_eq!(
        EntitySelector::parse_arg(&mut input).unwrap(),
        EntitySelector::ComplexSelector(EntitySelectors::AllEntities, "distance=..5".to_string())
    );
    assert!(!input.is_done());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PossiblyRelative<T> {
    Absolute(T),
    Relative(T), // current value + T
}

impl<T> PossiblyRelative<T>
where
    T: Add<Output = T> + Copy,
{
    pub fn get(&self, origanal: T) -> T {
        match self {
            Self::Absolute(num) => *num,
            Self::Relative(num) => *num + origanal,
        }
    }
}

impl<T> CommandArg for PossiblyRelative<T>
where
    T: CommandArg + Default,
{
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        if input.peek() == Some('~') {
            input.pop();
            if input.peek() == Some(' ') || input.peek().is_none() {
                Ok(PossiblyRelative::Relative(T::default()))
            } else {
                Ok(PossiblyRelative::Relative(T::parse_arg(input)?))
            }
        } else if input.peek() == Some(' ') || input.peek().is_none() {
            Err(CommandArgParseError::InvalidArgLength)
        } else {
            Ok(PossiblyRelative::Absolute(T::parse_arg(input)?))
        }
    }

    fn display() -> Parser {
        T::display()
    }
}

#[test]
fn test_possibly_relative() {
    let mut input = ParseInput::new("~".to_string());
    assert_eq!(
        PossiblyRelative::<i32>::parse_arg(&mut input).unwrap(),
        PossiblyRelative::Relative(0)
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("~1".to_string());
    assert_eq!(
        PossiblyRelative::<i32>::parse_arg(&mut input).unwrap(),
        PossiblyRelative::Relative(1)
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("~1.5".to_string());
    assert_eq!(
        PossiblyRelative::<f32>::parse_arg(&mut input).unwrap(),
        PossiblyRelative::Relative(1.5)
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("1".to_string());
    assert_eq!(
        PossiblyRelative::<i32>::parse_arg(&mut input).unwrap(),
        PossiblyRelative::Absolute(1)
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("1.5".to_string());
    assert_eq!(
        PossiblyRelative::<f32>::parse_arg(&mut input).unwrap(),
        PossiblyRelative::Absolute(1.5)
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("1.5 ".to_string());
    assert_eq!(
        PossiblyRelative::<f32>::parse_arg(&mut input).unwrap(),
        PossiblyRelative::Absolute(1.5)
    );
    assert!(!input.is_done());

    let mut input = ParseInput::new("1.5 2".to_string());
    assert_eq!(
        PossiblyRelative::<f32>::parse_arg(&mut input).unwrap(),
        PossiblyRelative::Absolute(1.5)
    );
    assert!(!input.is_done());

    let mut input = ParseInput::new("1.5 2 ".to_string());
    assert_eq!(
        PossiblyRelative::<f32>::parse_arg(&mut input).unwrap(),
        PossiblyRelative::Absolute(1.5)
    );
    assert!(!input.is_done());
}

impl<T: Default> Default for PossiblyRelative<T> {
    fn default() -> Self {
        PossiblyRelative::Absolute(T::default())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BlockPos {
    x: PossiblyRelative<i32>,
    y: PossiblyRelative<i32>,
    z: PossiblyRelative<i32>,
}

impl CommandArg for BlockPos {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        let x = PossiblyRelative::<i32>::parse_arg(input)?;
        input.skip_whitespace();
        let y = PossiblyRelative::<i32>::parse_arg(input)?;
        input.skip_whitespace();
        let z = PossiblyRelative::<i32>::parse_arg(input)?;

        Ok(BlockPos { x, y, z })
    }

    fn display() -> Parser {
        Parser::BlockPos
    }
}

#[test]
fn test_block_pos() {
    let mut input = ParseInput::new("~-1 2 3".to_string());
    assert_eq!(
        BlockPos::parse_arg(&mut input).unwrap(),
        BlockPos {
            x: PossiblyRelative::Relative(-1),
            y: PossiblyRelative::Absolute(2),
            z: PossiblyRelative::Absolute(3)
        }
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("-1 ~2 3 ".to_string());
    assert_eq!(
        BlockPos::parse_arg(&mut input).unwrap(),
        BlockPos {
            x: PossiblyRelative::Absolute(-1),
            y: PossiblyRelative::Relative(2),
            z: PossiblyRelative::Absolute(3)
        }
    );
    assert!(!input.is_done());

    let mut input = ParseInput::new("-1 2 ~3 4".to_string());
    assert_eq!(
        BlockPos::parse_arg(&mut input).unwrap(),
        BlockPos {
            x: PossiblyRelative::Absolute(-1),
            y: PossiblyRelative::Absolute(2),
            z: PossiblyRelative::Relative(3)
        }
    );
    assert!(!input.is_done());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ColumnPos {
    x: PossiblyRelative<i32>,
    y: PossiblyRelative<i32>,
    z: PossiblyRelative<i32>,
}

impl CommandArg for ColumnPos {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        let x = PossiblyRelative::<i32>::parse_arg(input)?;
        input.skip_whitespace();
        let y = PossiblyRelative::<i32>::parse_arg(input)?;
        input.skip_whitespace();
        let z = PossiblyRelative::<i32>::parse_arg(input)?;

        Ok(ColumnPos { x, y, z })
    }

    fn display() -> Parser {
        Parser::ColumnPos
    }
}

#[test]
fn test_column_pos() {
    let mut input = ParseInput::new("~-1 2 3".to_string());
    assert_eq!(
        ColumnPos::parse_arg(&mut input).unwrap(),
        ColumnPos {
            x: PossiblyRelative::Relative(-1),
            y: PossiblyRelative::Absolute(2),
            z: PossiblyRelative::Absolute(3)
        }
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("-1 ~2 3 ".to_string());
    assert_eq!(
        ColumnPos::parse_arg(&mut input).unwrap(),
        ColumnPos {
            x: PossiblyRelative::Absolute(1),
            y: PossiblyRelative::Relative(2),
            z: PossiblyRelative::Absolute(3)
        }
    );
    assert!(!input.is_done());

    let mut input = ParseInput::new("-1 2 ~3 4".to_string());
    assert_eq!(
        ColumnPos::parse_arg(&mut input).unwrap(),
        ColumnPos {
            x: PossiblyRelative::Absolute(1),
            y: PossiblyRelative::Absolute(2),
            z: PossiblyRelative::Relative(3)
        }
    );
    assert!(!input.is_done());
}

//
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    pub x: PossiblyRelative<f32>,
    pub y: PossiblyRelative<f32>,
    pub z: PossiblyRelative<f32>,
}

impl CommandArg for Vec3 {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        let x = PossiblyRelative::<f32>::parse_arg(input)?;
        input.skip_whitespace();
        let y = PossiblyRelative::<f32>::parse_arg(input)?;
        input.skip_whitespace();
        let z = PossiblyRelative::<f32>::parse_arg(input)?;

        Ok(Vec3 { x, y, z })
    }

    fn display() -> Parser {
        Parser::Vec3
    }
}

#[test]
fn test_vec3() {
    let mut input = ParseInput::new("~-1.5 2.5 3.5".to_string());
    assert_eq!(
        Vec3::parse_arg(&mut input).unwrap(),
        Vec3 {
            x: PossiblyRelative::Relative(-1.5),
            y: PossiblyRelative::Absolute(2.5),
            z: PossiblyRelative::Absolute(3.5)
        }
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("-1.5 ~2.5 3.5 ".to_string());
    assert_eq!(
        Vec3::parse_arg(&mut input).unwrap(),
        Vec3 {
            x: PossiblyRelative::Absolute(-1.5),
            y: PossiblyRelative::Relative(2.5),
            z: PossiblyRelative::Absolute(3.5)
        }
    );
    assert!(!input.is_done());

    let mut input = ParseInput::new("-1.5 2.5 ~3.5 4.5".to_string());
    assert_eq!(
        Vec3::parse_arg(&mut input).unwrap(),
        Vec3 {
            x: PossiblyRelative::Absolute(-1.5),
            y: PossiblyRelative::Absolute(2.5),
            z: PossiblyRelative::Relative(3.5)
        }
    );
    assert!(!input.is_done());
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    pub x: PossiblyRelative<f32>,
    pub y: PossiblyRelative<f32>,
}

impl CommandArg for Vec2 {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        let x = PossiblyRelative::<f32>::parse_arg(input)?;
        input.skip_whitespace();
        let y = PossiblyRelative::<f32>::parse_arg(input)?;

        Ok(Vec2 { x, y })
    }

    fn display() -> Parser {
        Parser::Vec2
    }
}

#[test]
fn test_vec2() {
    let mut input = ParseInput::new("~-1.5 2.5".to_string());
    assert_eq!(
        Vec2::parse_arg(&mut input).unwrap(),
        Vec2 {
            x: PossiblyRelative::Relative(-1.5),
            y: PossiblyRelative::Absolute(2.5),
        }
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("-1.5 ~2.5 ".to_string());
    assert_eq!(
        Vec2::parse_arg(&mut input).unwrap(),
        Vec2 {
            x: PossiblyRelative::Absolute(-1.5),
            y: PossiblyRelative::Relative(2.5),
        }
    );
    assert!(!input.is_done());

    let mut input = ParseInput::new("-1.5 2.5 3.5".to_string());
    assert_eq!(
        Vec2::parse_arg(&mut input).unwrap(),
        Vec2 {
            x: PossiblyRelative::Absolute(-1.5),
            y: PossiblyRelative::Absolute(2.5),
        }
    );
    assert!(!input.is_done());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ChatColor {
    Black,
    DarkBlue,
    DarkGreen,
    DarkAqua,
    DarkRed,
    DarkPurple,
    Gold,
    Gray,
    DarkGray,
    Blue,
    Green,
    Aqua,
    Red,
    LightPurple,
    Yellow,
    #[default]
    White,
    Reset,
}

impl CommandArg for ChatColor {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        if input.match_next("black") {
            Ok(ChatColor::Black)
        } else if input.match_next("dark_blue") {
            Ok(ChatColor::DarkBlue)
        } else if input.match_next("dark_green") {
            Ok(ChatColor::DarkGreen)
        } else if input.match_next("dark_aqua") {
            Ok(ChatColor::DarkAqua)
        } else if input.match_next("dark_red") {
            Ok(ChatColor::DarkRed)
        } else if input.match_next("dark_purple") {
            Ok(ChatColor::DarkPurple)
        } else if input.match_next("gold") {
            Ok(ChatColor::Gold)
        } else if input.match_next("gray") {
            Ok(ChatColor::Gray)
        } else if input.match_next("dark_gray") {
            Ok(ChatColor::DarkGray)
        } else if input.match_next("blue") {
            Ok(ChatColor::Blue)
        } else if input.match_next("green") {
            Ok(ChatColor::Green)
        } else if input.match_next("aqua") {
            Ok(ChatColor::Aqua)
        } else if input.match_next("red") {
            Ok(ChatColor::Red)
        } else if input.match_next("light_purple") {
            Ok(ChatColor::LightPurple)
        } else if input.match_next("yellow") {
            Ok(ChatColor::Yellow)
        } else if input.match_next("white") {
            Ok(ChatColor::White)
        } else if input.match_next("reset") {
            Ok(ChatColor::Reset)
        } else {
            Err(CommandArgParseError::InvalidArgument(
                "chat_color".to_string(),
                "not a valid chat color".to_string(),
            ))
        }
    }

    fn display() -> Parser {
        Parser::Color
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Angle(f32);

impl CommandArg for Angle {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        let angle = f32::parse_arg(input)?;

        Ok(Angle(angle))
    }

    fn display() -> Parser {
        Parser::Angle
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rotation(Vec2);

impl CommandArg for Rotation {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        let vec2 = Vec2::parse_arg(input)?;

        Ok(Rotation(vec2))
    }

    fn display() -> Parser {
        Parser::Rotation
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ScoreHolder {
    Entity(String), // TODO: EntitySelector proper
    #[default]
    All,
}

impl CommandArg for ScoreHolder {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        if input.peek() == Some('*') {
            Ok(ScoreHolder::All)
        } else {
            let name = match input.pop_to_next_whitespace_or_end() {
                Some(name) => name,
                None => {
                    return Err(CommandArgParseError::InvalidArgument(
                        "score_holder".to_string(),
                        "expected a score holder".to_string(),
                    ))
                }
            };
            Ok(ScoreHolder::Entity(name))
        }
    }

    fn display() -> Parser {
        Parser::ScoreHolder {
            allow_multiple: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Swizzle {
    pub x: bool,
    pub y: bool,
    pub z: bool,
}

impl CommandArg for Swizzle {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        let mut swizzle = Swizzle::default();
        while let Some(c) = input.peek() {
            match c {
                'x' => swizzle.x = true,
                'y' => swizzle.y = true,
                'z' => swizzle.z = true,
                _ => break,
            }
            input.pop();
        }

        Ok(swizzle)
    }

    fn display() -> Parser {
        Parser::Swizzle
    }
}

#[test]
fn test_swizzle() {
    let mut input = ParseInput::new("xyzzzz");
    let swizzle = Swizzle::parse_arg(&mut input).unwrap();
    assert_eq!(
        swizzle,
        Swizzle {
            x: true,
            y: true,
            z: true
        }
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("xzy");
    let swizzle = Swizzle::parse_arg(&mut input).unwrap();
    assert_eq!(
        swizzle,
        Swizzle {
            x: true,
            y: true,
            z: true
        }
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("x");
    let swizzle = Swizzle::parse_arg(&mut input).unwrap();
    assert_eq!(
        swizzle,
        Swizzle {
            x: true,
            y: false,
            z: false
        }
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("x y z zy xyz");
    let swizzle_a = Swizzle::parse_arg(&mut input).unwrap();
    let swizzle_b = Swizzle::parse_arg(&mut input).unwrap();
    let swizzle_c = Swizzle::parse_arg(&mut input).unwrap();
    let swizzle_d = Swizzle::parse_arg(&mut input).unwrap();
    let swizzle_e = Swizzle::parse_arg(&mut input).unwrap();
    assert_eq!(
        swizzle_a,
        Swizzle {
            x: true,
            y: false,
            z: false
        }
    );
    assert_eq!(
        swizzle_b,
        Swizzle {
            x: false,
            y: true,
            z: false
        }
    );
    assert_eq!(
        swizzle_c,
        Swizzle {
            x: false,
            y: false,
            z: true
        }
    );
    assert_eq!(
        swizzle_d,
        Swizzle {
            x: false,
            y: true,
            z: true
        }
    );
    assert_eq!(
        swizzle_e,
        Swizzle {
            x: true,
            y: true,
            z: true
        }
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InventorySlot(u32);

impl CommandArg for InventorySlot {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        let slot = u32::parse_arg(input)?;

        Ok(InventorySlot(slot))
    }

    fn display() -> Parser {
        Parser::ItemSlot
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EntityAnchor {
    #[default]
    Eyes,
    Feet,
}

impl CommandArg for EntityAnchor {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        if input.match_next("eyes") {
            Ok(EntityAnchor::Eyes)
        } else if input.match_next("feet") {
            Ok(EntityAnchor::Feet)
        } else {
            Err(CommandArgParseError::InvalidArgument(
                "entity_anchor".to_string(),
                "not a valid entity anchor".to_string(),
            ))
        }
    }

    fn display() -> Parser {
        Parser::EntityAnchor
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IntRange {
    pub min: i32,
    pub max: i32,
}

impl CommandArg for IntRange {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        let input =
            input
                .pop_to_next_whitespace_or_end()
                .ok_or(CommandArgParseError::InvalidArgument(
                    "int_range".to_string(),
                    "expected a range".to_string(),
                ))?;
        let mut input = input.split("..");

        let min = input.next().unwrap().parse::<i32>().map_err(|_| {
            CommandArgParseError::InvalidArgument(
                "int_range max".to_string(),
                input.clone().collect::<Vec<&str>>().join(".."),
            )
        })?;
        let max = input.next().unwrap().parse::<i32>().map_err(|_| {
            CommandArgParseError::InvalidArgument(
                "int_range min".to_string(),
                input.clone().collect::<Vec<&str>>().join(".."),
            )
        })?;

        Ok(IntRange { min, max })
    }

    fn display() -> Parser {
        Parser::IntRange
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FloatRange {
    pub min: f32,
    pub max: f32,
}

impl CommandArg for FloatRange {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        let input =
            input
                .pop_to_next_whitespace_or_end()
                .ok_or(CommandArgParseError::InvalidArgument(
                    "float_range".to_string(),
                    "expected a range".to_string(),
                ))?;
        let mut input = input.split("..");

        let min = input.next().unwrap().parse::<f32>().map_err(|_| {
            CommandArgParseError::InvalidArgument(
                "float_range max".to_string(),
                input.clone().collect::<Vec<&str>>().join(".."),
            )
        })?;
        let max = input.next().unwrap().parse::<f32>().map_err(|_| {
            CommandArgParseError::InvalidArgument(
                "float_range min".to_string(),
                input.clone().collect::<Vec<&str>>().join(".."),
            )
        })?;

        Ok(FloatRange { min, max })
    }

    fn display() -> Parser {
        Parser::FloatRange
    }
}

#[test]
fn test_ranges() {
    let mut input = ParseInput::new("1.0..2.0".to_string());
    let range = FloatRange::parse_arg(&mut input).unwrap();
    assert_eq!(range.min, 1.0);
    assert_eq!(range.max, 2.0);

    let mut input = ParseInput::new("1..2".to_string());
    let range = IntRange::parse_arg(&mut input).unwrap();
    assert_eq!(range.min, 1);
    assert_eq!(range.max, 2);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dimension {
    #[default]
    Overworld,
    Nether,
    End,
}

impl CommandArg for Dimension {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        if input.match_next("overworld") {
            Ok(Dimension::Overworld)
        } else if input.match_next("nether") {
            Ok(Dimension::Nether)
        } else if input.match_next("end") {
            Ok(Dimension::End)
        } else {
            Err(CommandArgParseError::InvalidArgument(
                "dimension".to_string(),
                "not a valid dimension".to_string(),
            ))
        }
    }

    fn display() -> Parser {
        Parser::Dimension
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GameMode {
    #[default]
    Survival,
    Creative,
    Adventure,
    Spectator,
}

impl CommandArg for GameMode {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        if input.match_next("survival") {
            Ok(GameMode::Survival)
        } else if input.match_next("creative") {
            Ok(GameMode::Creative)
        } else if input.match_next("adventure") {
            Ok(GameMode::Adventure)
        } else if input.match_next("spectator") {
            Ok(GameMode::Spectator)
        } else {
            Err(CommandArgParseError::InvalidArgument(
                "game_mode".to_string(),
                "not a valid game mode".to_string(),
            ))
        }
    }

    fn display() -> Parser {
        Parser::GameMode
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Time {
    Ticks(f32),
    Second(f32),
    Day(f32),
}

impl CommandArg for Time {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        let mut number_str = String::new();
        while let Some(c) = input.pop() {
            match c {
                't' => {
                    return Ok(Time::Ticks(number_str.parse::<f32>().map_err(|_| {
                        CommandArgParseError::InvalidArgument(
                            "time".to_string(),
                            "not a valid time".to_string(),
                        )
                    })?));
                }
                's' => {
                    return Ok(Time::Second(number_str.parse::<f32>().map_err(|_| {
                        CommandArgParseError::InvalidArgument(
                            "time".to_string(),
                            "not a valid time".to_string(),
                        )
                    })?));
                }
                'd' => {
                    return Ok(Time::Day(number_str.parse::<f32>().map_err(|_| {
                        CommandArgParseError::InvalidArgument(
                            "time".to_string(),
                            "not a valid time".to_string(),
                        )
                    })?));
                }
                _ => {
                    number_str.push(c);
                }
            }
        }
        if !number_str.is_empty() {
            return Ok(Time::Ticks(number_str.parse::<f32>().map_err(|_| {
                CommandArgParseError::InvalidArgument(
                    "time".to_string(),
                    "not a valid time".to_string(),
                )
            })?));
        }

        Err(CommandArgParseError::InvalidArgument(
            "time".to_string(),
            "not a valid time".to_string(),
        ))
    }

    fn display() -> Parser {
        Parser::Time
    }
}

#[test]
fn test_time() {
    let mut input = ParseInput::new("42.31t".to_string());
    let time = Time::parse_arg(&mut input).unwrap();
    assert_eq!(time, Time::Ticks(42.31));

    let mut input = ParseInput::new("42.31".to_string());
    let time = Time::parse_arg(&mut input).unwrap();
    assert_eq!(time, Time::Ticks(42.31));

    let mut input = ParseInput::new("1239.72s".to_string());
    let time = Time::parse_arg(&mut input).unwrap();
    assert_eq!(time, Time::Second(1239.72));

    let mut input = ParseInput::new("133.1d".to_string());
    let time = Time::parse_arg(&mut input).unwrap();
    assert_eq!(time, Time::Day(133.1));
}
