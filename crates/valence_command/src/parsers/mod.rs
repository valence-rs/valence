//! A collection of parses for use in command argument nodes.

pub mod angle;
pub mod block_pos;
pub mod bool;
pub mod colour;
pub mod column_pos;
pub mod entity_anchor;
pub mod entity_selector;
pub mod gamemode;
pub mod inventory_slot;
pub mod numbers;
pub mod rotation;
pub mod score_holder;
pub mod strings;
pub mod swizzle;
pub mod time;
pub mod vec2;
pub mod vec3;

use std::ops::Add;

use thiserror::Error;
use valence_server::protocol::packets::play::command_tree_s2c::Parser;

pub trait CommandArg: Sized {
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
        Self {
            input: input.into(),
            cursor: 0,
        }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbsoluteOrRelative<T> {
    Absolute(T),
    Relative(T), // current value + T
}

impl<T> AbsoluteOrRelative<T>
where
    T: Add<Output = T> + Copy,
{
    pub fn get(&self, original: T) -> T {
        match self {
            Self::Absolute(num) => *num,
            Self::Relative(num) => *num + original,
        }
    }
}

impl<T> CommandArg for AbsoluteOrRelative<T>
where
    T: CommandArg + Default,
{
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        if input.peek() == Some('~') {
            input.pop();
            if input.peek() == Some(' ') || input.peek().is_none() {
                Ok(AbsoluteOrRelative::Relative(T::default()))
            } else {
                Ok(AbsoluteOrRelative::Relative(T::parse_arg(input)?))
            }
        } else if input.peek() == Some(' ') || input.peek().is_none() {
            Err(CommandArgParseError::InvalidArgLength)
        } else {
            Ok(AbsoluteOrRelative::Absolute(T::parse_arg(input)?))
        }
    }

    fn display() -> Parser {
        T::display()
    }
}

#[test]
fn test_absolute_or_relative() {
    let mut input = ParseInput::new("~".to_string());
    assert_eq!(
        AbsoluteOrRelative::<i32>::parse_arg(&mut input).unwrap(),
        AbsoluteOrRelative::Relative(0)
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("~1".to_string());
    assert_eq!(
        AbsoluteOrRelative::<i32>::parse_arg(&mut input).unwrap(),
        AbsoluteOrRelative::Relative(1)
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("~1.5".to_string());
    assert_eq!(
        AbsoluteOrRelative::<f32>::parse_arg(&mut input).unwrap(),
        AbsoluteOrRelative::Relative(1.5)
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("1".to_string());
    assert_eq!(
        AbsoluteOrRelative::<i32>::parse_arg(&mut input).unwrap(),
        AbsoluteOrRelative::Absolute(1)
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("1.5".to_string());
    assert_eq!(
        AbsoluteOrRelative::<f32>::parse_arg(&mut input).unwrap(),
        AbsoluteOrRelative::Absolute(1.5)
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("1.5 ".to_string());
    assert_eq!(
        AbsoluteOrRelative::<f32>::parse_arg(&mut input).unwrap(),
        AbsoluteOrRelative::Absolute(1.5)
    );
    assert!(!input.is_done());

    let mut input = ParseInput::new("1.5 2".to_string());
    assert_eq!(
        AbsoluteOrRelative::<f32>::parse_arg(&mut input).unwrap(),
        AbsoluteOrRelative::Absolute(1.5)
    );
    assert!(!input.is_done());

    let mut input = ParseInput::new("1.5 2 ".to_string());
    assert_eq!(
        AbsoluteOrRelative::<f32>::parse_arg(&mut input).unwrap(),
        AbsoluteOrRelative::Absolute(1.5)
    );
    assert!(!input.is_done());
}

impl<T: Default> Default for AbsoluteOrRelative<T> {
    fn default() -> Self {
        AbsoluteOrRelative::Absolute(T::default())
    }
}
