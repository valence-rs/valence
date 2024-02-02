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
pub struct ParseInput<'a>(&'a str);

impl<'a> ParseInput<'a> {
    fn advance(&mut self) {
        self.advance_n_chars(1);
    }

    fn advance_n_chars(&mut self, n: usize) {
        if self.is_done() {
            return;
        }
        match self.0.char_indices().nth(n) {
            Some((len, _)) => {
                self.0 = &self.0[len..];
            }
            None => {
                self.0 = &self.0[self.0.len()..];
            }
        }
    }

    fn advance_n_bytes(&mut self, n: usize) {
        if self.is_done() {
            return;
        }
        self.0 = &self.0[n..];
    }
    pub fn new(input: &'a str) -> Self {
        ParseInput(input)
    }

    /// Returns the next character without advancing the input
    pub fn peek(&self) -> Option<char> {
        self.0.chars().next()
    }

    /// Returns the next n characters without advancing the input
    pub fn peek_n(&self, n: usize) -> &'a str {
        self.0
            .char_indices()
            .nth(n)
            .map(|(idx, _)| &self.0[..idx])
            .unwrap_or(self.0)
    }

    /// Returns the next word without advancing the input
    pub fn peek_word(&self) -> &'a str {
        self.0
            .char_indices()
            .find(|(_, c)| c.is_whitespace())
            .map(|(idx, _)| &self.0[..idx])
            .unwrap_or(self.0)
    }

    /// Checks if the input is empty
    pub fn is_done(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the next character and advances the input
    pub fn pop(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.advance();
        Some(c)
    }

    /// Returns the next n characters and advances the input
    pub fn pop_n(&mut self, n: usize) -> &str {
        let s = self.peek_n(n);
        self.advance_n_bytes(s.len());
        s
    }

    /// Returns the next word and advances the input
    pub fn pop_word(&mut self) -> &str {
        let s = self.peek_word();
        self.advance_n_bytes(s.len());
        s
    }

    /// Returns the rest of the input and advances the input
    pub fn pop_all(&mut self) -> Option<&str> {
        let s = self.0;
        self.advance_n_bytes(self.0.len());
        Some(s)
    }

    /// Returns the next word and advances the input
    pub fn pop_to_next(&mut self, c: char) -> Option<&str> {
        let pos = self.0.find(c)?;
        let s = &self.0[..pos];
        self.advance_n_bytes(pos);
        Some(s)
    }

    /// Matches the case-insensitive string and advances the input if it matches
    pub fn match_next(&mut self, string: &str) -> bool {
        if self
            .0
            .to_lowercase()
            .starts_with(string.to_lowercase().as_str())
        {
            self.advance_n_bytes(string.len());
            true
        } else {
            false
        }
    }

    /// Skip all whitespace at the front of the input
    pub fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Set the inner string
    pub fn into_inner(self) -> &'a str {
        self.0
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

#[test]
fn test_parse_input() {
    let mut input = ParseInput::new("The QuIck brown FOX jumps over the lazy dog");
    assert_eq!(input.peek(), Some('T'));
    assert_eq!(input.peek_n(0), "");
    assert_eq!(input.peek_n(1), "T");
    assert_eq!(input.peek_n(2), "Th");
    assert_eq!(input.peek_n(3), "The");

    assert_eq!(input.peek_word(), "The");
    input.pop_word();
    input.skip_whitespace();
    assert_eq!(input.peek_word(), "QuIck");

    assert!(input.match_next("quick"));
    input.pop();
    assert_eq!(input.peek_word(), "brown");

    assert!(input.match_next("brown fox"));
    assert_eq!(input.pop_all(), Some(" jumps over the lazy dog"));
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
