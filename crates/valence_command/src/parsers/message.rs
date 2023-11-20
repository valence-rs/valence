use super::Parser;
use crate::parsers::{CommandArg, CommandArgParseError, EntitySelector, ParseInput};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    pub message: String,
    pub selectors: Vec<MessageSelector>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageSelector {
    pub start: u32,
    pub end: u32,
    pub selector: EntitySelector,
}

impl CommandArg for Message {
    fn parse_arg(input: &mut ParseInput) -> Result<Self, CommandArgParseError> {
        input.skip_whitespace();

        let message = input.clone().into_inner().to_string();
        let mut selectors: Vec<MessageSelector> = Vec::new();

        let mut i = 0u32;
        while let Some(c) = input.peek() {
            if c == '@' {
                let start = i;
                let length_before = input.len();
                let selector = EntitySelector::parse_arg(input)?;
                i += length_before as u32 - input.len() as u32;
                selectors.push(MessageSelector {
                    start,
                    end: i,
                    selector,
                });
            } else {
                i += 1;
                input.advance();
            }
        }

        Ok(Message { message, selectors })
    }

    fn display() -> Parser {
        Parser::Message
    }
}

#[test]
fn test_message() {
    use crate::parsers::entity_selector::EntitySelectors;

    let mut input = ParseInput::new("Hello @e");
    assert_eq!(
        Message::parse_arg(&mut input).unwrap(),
        Message {
            message: "Hello @e".to_string(),
            selectors: vec![MessageSelector {
                start: 6,
                end: 8,
                selector: EntitySelector::SimpleSelector(EntitySelectors::AllEntities)
            }]
        }
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("@p say hi to @a");
    assert_eq!(
        Message::parse_arg(&mut input).unwrap(),
        Message {
            message: "@p say hi to @a".to_string(),
            selectors: vec![
                MessageSelector {
                    start: 0,
                    end: 2,
                    selector: EntitySelector::SimpleSelector(EntitySelectors::NearestPlayer)
                },
                MessageSelector {
                    start: 13,
                    end: 15,
                    selector: EntitySelector::SimpleSelector(EntitySelectors::AllPlayers)
                },
            ]
        }
    );
    assert!(input.is_done());

    let mut input = ParseInput::new("say hi to nearby players @p[distance=..5]");
    assert_eq!(
        Message::parse_arg(&mut input).unwrap(),
        Message {
            message: "say hi to nearby players @p[distance=..5]".to_string(),
            selectors: vec![MessageSelector {
                start: 25,
                end: 41,
                selector: EntitySelector::ComplexSelector(
                    EntitySelectors::NearestPlayer,
                    "distance=..5".to_string()
                )
            },]
        }
    );
    assert!(input.is_done());
}
