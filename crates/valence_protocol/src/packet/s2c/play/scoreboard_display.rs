use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct ScoreboardDisplayS2c<'a> {
    pub position: Position,
    pub score_name: &'a str,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Position {
    List,
    Sidebar,
    BelowName,
    Team(u8),
}

impl Encode for Position {
    fn encode(&self, w: impl std::io::Write) -> anyhow::Result<()> {
        match self {
            Position::List => 0u8.encode(w),
            Position::Sidebar => 1u8.encode(w),
            Position::BelowName => 2u8.encode(w),
            Position::Team(value) => {
                if value > &15 {
                    return Err(anyhow::anyhow!("Invalid scoreboard display position"));
                }

                (3 + value).encode(w)
            }
        }
    }
}

impl<'a> Decode<'a> for Position {
    fn decode(r: &mut &'a [u8]) -> anyhow::Result<Self> {
        let value = u8::decode(r)?;
        match value {
            0 => Ok(Position::List),
            1 => Ok(Position::Sidebar),
            2 => Ok(Position::BelowName),
            3..=15 => Ok(Position::Team(value - 3)),
            _ => Err(anyhow::anyhow!("Invalid scoreboard display position")),
        }
    }
}
