use valence_server::protocol::packets::play::command_tree_s2c::Parser;

use crate::parsers::{CommandArg, CommandArgParseError, ParseInput};

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
    // we want to get either a simple string [`@e`, `@a`, `@p`, `@r`,
    // `<player_name>`] or a full selector: [`@e[<selector>]`, `@a[<selector>]`,
    // `@p[<selector>]`, `@r[<selector>]`] the selectors can have spaces in
    // them, so we need to be careful
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();
        let mut s = String::new();
        let mut selector = None;
        while let Some(c) = input.peek() {
            match c {
                '@' => {
                    input.pop();
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
                    input.pop();
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
                    return Ok(EntitySelector::SimpleSelector(
                        EntitySelectors::SinglePlayer(String::parse_arg(input)?),
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
