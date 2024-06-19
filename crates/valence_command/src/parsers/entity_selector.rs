use super::Parser;
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
        let mut simple_selector = None;
        while let Some(c) = input.peek() {
            match c {
                '@' => {
                    input.pop(); // pop the '@'
                    match input.pop() {
                        Some('e') => simple_selector = Some(EntitySelectors::AllEntities),
                        Some('a') => simple_selector = Some(EntitySelectors::AllPlayers),
                        Some('p') => simple_selector = Some(EntitySelectors::NearestPlayer),
                        Some('r') => simple_selector = Some(EntitySelectors::RandomPlayer),
                        Some('s') => simple_selector = Some(EntitySelectors::SelfPlayer),
                        _ => {
                            return Err(CommandArgParseError::InvalidArgument {
                                expected: "entity selector".to_owned(),
                                got: c.to_string(),
                            })
                        }
                    }
                    if input.peek() != Some('[') {
                        // if there's no complex selector, we're done
                        return Ok(EntitySelector::SimpleSelector(simple_selector.unwrap()));
                    }
                }
                '[' => {
                    input.pop();
                    if simple_selector.is_none() {
                        return Err(CommandArgParseError::InvalidArgument {
                            expected: "entity selector".to_owned(),
                            got: c.to_string(),
                        });
                    }
                    let mut s = String::new();
                    while let Some(c) = input.pop() {
                        if c == ']' {
                            return Ok(EntitySelector::ComplexSelector(
                                simple_selector.unwrap(),
                                s.trim().to_owned(),
                            ));
                        }

                        s.push(c);
                    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_selector() {
        let mut input = ParseInput::new("@e");
        assert_eq!(
            EntitySelector::parse_arg(&mut input).unwrap(),
            EntitySelector::SimpleSelector(EntitySelectors::AllEntities)
        );
        assert!(input.is_done());

        let mut input = ParseInput::new("@e[distance=..5]");
        assert_eq!(
            EntitySelector::parse_arg(&mut input).unwrap(),
            EntitySelector::ComplexSelector(
                EntitySelectors::AllEntities,
                "distance=..5".to_owned()
            )
        );
        assert!(input.is_done());

        let mut input = ParseInput::new("@s[distance=..5");
        assert!(EntitySelector::parse_arg(&mut input).is_err());
        assert!(input.is_done());

        let mut input = ParseInput::new("@r[distance=..5] hello");
        assert_eq!(
            EntitySelector::parse_arg(&mut input).unwrap(),
            EntitySelector::ComplexSelector(
                EntitySelectors::RandomPlayer,
                "distance=..5".to_owned()
            )
        );
        assert!(!input.is_done());

        let mut input = ParseInput::new("@p[distance=..5]hello");
        assert_eq!(
            EntitySelector::parse_arg(&mut input).unwrap(),
            EntitySelector::ComplexSelector(
                EntitySelectors::NearestPlayer,
                "distance=..5".to_owned()
            )
        );
        assert!(!input.is_done());

        let mut input = ParseInput::new("@e[distance=..5] hello world");
        assert_eq!(
            EntitySelector::parse_arg(&mut input).unwrap(),
            EntitySelector::ComplexSelector(
                EntitySelectors::AllEntities,
                "distance=..5".to_owned()
            )
        );
        assert!(!input.is_done());

        let mut input = ParseInput::new("@e[distance=..5]hello world");
        assert_eq!(
            EntitySelector::parse_arg(&mut input).unwrap(),
            EntitySelector::ComplexSelector(
                EntitySelectors::AllEntities,
                "distance=..5".to_owned()
            )
        );
        assert!(!input.is_done());
    }
}
