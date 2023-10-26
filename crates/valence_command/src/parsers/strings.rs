use bevy_derive::Deref;
use valence_server::protocol::packets::play::command_tree_s2c::StringArg;

use super::Parser;
use crate::parsers::{CommandArg, CommandArgParseError, ParseInput};

impl CommandArg for String {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        Ok(input.pop_word().to_string())
    }

    fn display() -> Parser {
        Parser::String(StringArg::SingleWord)
    }
}

#[test]
fn test_string() {
    let mut input = ParseInput::new("hello world");
    assert_eq!("hello", String::parse_arg(&mut input).unwrap());
    assert_eq!("world", String::parse_arg(&mut input).unwrap());
    assert!(input.is_done());
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deref)]
pub struct GreedyString(pub String);

impl CommandArg for GreedyString {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        Ok(GreedyString(
            match input.pop_all() {
                Some(s) => s,
                None => return Err(CommandArgParseError::InvalidArgLength),
            }
            .to_string(),
        ))
    }

    fn display() -> Parser {
        Parser::String(StringArg::GreedyPhrase)
    }
}

#[test]
fn test_greedy_string() {
    let mut input = ParseInput::new("hello world");
    assert_eq!(
        "hello world",
        GreedyString::parse_arg(&mut input).unwrap().0
    );
    assert!(input.is_done());
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Deref)]
pub struct QuotableString(pub String);

impl CommandArg for QuotableString {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        match input.peek() {
            Some('"') => {
                input.pop();
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
    let mut input = ParseInput::new("\"hello world\"");
    assert_eq!(
        "hello world",
        QuotableString::parse_arg(&mut input).unwrap().0
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("\"hello w\"orld");
    assert_eq!("hello w", QuotableString::parse_arg(&mut input).unwrap().0);
    assert!(!input.is_done());

    let mut input = ParseInput::new("hello world\"");
    assert_eq!("hello", QuotableString::parse_arg(&mut input).unwrap().0);
    assert!(!input.is_done());
}
