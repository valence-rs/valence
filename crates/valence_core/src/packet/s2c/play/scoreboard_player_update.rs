use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct ScoreboardPlayerUpdateS2c<'a> {
    pub entity_name: &'a str,
    pub action: Action<'a>,
}

#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub enum Action<'a> {
    Update {
        objective_name: &'a str,
        objective_score: VarInt,
    },
    Remove {
        objective_name: &'a str,
    },
}
