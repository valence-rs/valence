//! A collection of parses for use in command argument nodes.
pub mod angle;
pub mod block_pos;
pub mod bool;
pub mod color;
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

pub use block_pos::BlockPos;
pub use column_pos::ColumnPos;
pub use entity_anchor::EntityAnchor;
pub use entity_selector::EntitySelector;
pub use inventory_slot::InventorySlot;
pub use rotation::Rotation;
pub use score_holder::ScoreHolder;
pub use strings::{GreedyString, QuotableString};
pub use swizzle::Swizzle;
use thiserror::Error;
pub use time::Time;
use tracing::error;
pub(crate) use valence_server::protocol::packets::play::command_tree_s2c::Parser;
pub use vec2::Vec2;
pub use vec3::Vec3;

pub trait CommandArg: Sized {
    fn arg_from_str(string: &str) -> Result<Self, CommandArgParseError> {
        Self::parse_arg(&mut ParseInput::new(string))
    }

    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError>;
    /// what will the client be sent
    fn display() -> Parser;
}

///
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseInput<'a> {
    input: &'a str,
    traversed: usize,
}

impl<'a> ParseInput<'a> {
    pub fn new(input: impl Into<&'a str>) -> Self {
        ParseInput {
            input: input.into(),
            traversed: 0,
        }
    }
    pub fn peek(&self) -> Option<char> {
        self.input.chars().next()
    }

    pub fn peek_n(&self, n: usize) -> Option<&str> {
        if n == 0 {
            error!("peek_n(0) called, don't do that.");
            return None; // never peek 0 chars
        }
        if n > self.input.chars().count() {
            return Some(self.input);
        }
        Some(&self.input[..=self.input.char_indices().nth(n - 1)?.0])
    }

    pub fn peek_word(&self) -> String {
        let iter = self.input.chars();
        let mut word = String::new();
        for c in iter {
            if c.is_whitespace() {
                break;
            } else {
                word.push(c);
            }
        }
        word
    }

    pub fn is_done(&self) -> bool {
        self.input.is_empty()
    }

    pub fn advance(&mut self) {
        self.advance_n_chars(1);
    }

    pub fn advance_n_chars(&mut self, n: usize) {
        if self.is_done() {
            return;
        }
        match self.input.char_indices().nth(n) {
            Some((len, _)) => {
                self.input = &self.input[len..];
                self.traversed += n;
            }
            None => {
                self.traversed += self.input.chars().count();
                self.input = "";
            }
        }
    }

    pub fn advance_n_bytes(&mut self, n: usize) {
        if self.is_done() {
            return;
        }
        self.advance_n_chars(self.input[..n].chars().count());
    }
    pub fn advance_to_next(&mut self, c: char) {
        if let Some(pos) = self.input.find(c) {
            self.advance_n_bytes(pos - 1);
        } else {
            self.input = "";
        }
    }

    pub fn advance_to_next_non_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    pub fn pop(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.advance();
        Some(c)
    }

    pub fn pop_n(&mut self, n: usize) -> Option<&str> {
        if n == 0 {
            return None; // never pop 0 chars
        }
        let s = &self.input[..self.input.char_indices().nth(n)?.0];
        self.advance_n_chars(n);
        Some(s)
    }

    pub fn pop_word(&mut self) -> String {
        let word = self.peek_word();
        self.advance_n_bytes(word.len());
        word
    }

    pub fn pop_all(&mut self) -> Option<&str> {
        let s = self.input;
        self.advance_n_bytes(self.input.len());
        Some(s)
    }

    pub fn pop_to_next(&mut self, c: char) -> Option<&str> {
        let pos = self.input.find(c)?;
        let s = &self.input[..pos];
        self.advance_n_bytes(pos);
        Some(s)
    }

    pub fn match_next(&mut self, string: &str) -> bool {
        if self.input.to_lowercase().starts_with(string) {
            self.advance_n_bytes(string.len());
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

    pub fn traversed(&self) -> usize {
        self.traversed
    }
}

#[derive(Debug, Error)]
pub enum CommandArgParseError {
    // these should be player facing and not disclose internal information
    #[error("invalid argument, expected {expected} got {got}")] // e.g. "integer" number
    InvalidArgument { expected: String, got: String },
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
            input.advance();
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
    let mut input = ParseInput::new("~");
    assert_eq!(
        AbsoluteOrRelative::<i32>::parse_arg(&mut input).unwrap(),
        AbsoluteOrRelative::Relative(0)
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("~1");
    assert_eq!(
        AbsoluteOrRelative::<i32>::parse_arg(&mut input).unwrap(),
        AbsoluteOrRelative::Relative(1)
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("~1.5");
    assert_eq!(
        AbsoluteOrRelative::<f32>::parse_arg(&mut input).unwrap(),
        AbsoluteOrRelative::Relative(1.5)
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("1");
    assert_eq!(
        AbsoluteOrRelative::<i32>::parse_arg(&mut input).unwrap(),
        AbsoluteOrRelative::Absolute(1)
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("1.5 ");
    assert_eq!(
        AbsoluteOrRelative::<f32>::parse_arg(&mut input).unwrap(),
        AbsoluteOrRelative::Absolute(1.5)
    );
    assert!(!input.is_done());

    let mut input = ParseInput::new("1.5 2");
    assert_eq!(
        AbsoluteOrRelative::<f32>::parse_arg(&mut input).unwrap(),
        AbsoluteOrRelative::Absolute(1.5)
    );
    assert!(!input.is_done());
    assert_eq!(
        AbsoluteOrRelative::<f32>::parse_arg(&mut input).unwrap(),
        AbsoluteOrRelative::Absolute(2.0)
    );
    assert!(input.is_done());
}

impl<T: Default> Default for AbsoluteOrRelative<T> {
    fn default() -> Self {
        AbsoluteOrRelative::Absolute(T::default())
    }
}
