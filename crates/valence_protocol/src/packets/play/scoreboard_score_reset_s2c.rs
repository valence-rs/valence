use crate::{Bounded, Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct ScoreboardScoreResetS2c<'a> {
    pub entity_name: Bounded<&'a str, 32000>,
    pub objective_name: Bounded<&'a str, 32000>,
}