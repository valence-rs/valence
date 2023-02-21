use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct ScoreboardPlayerUpdateS2c<'a> {
    pub entity_name: &'a str,
    pub action: Action<'a>,
}

// TODO: this looks wrong.
#[derive(Clone, PartialEq, Debug, Encode, Decode)]
pub enum Action<'a> {
    Create {
        objective_value: &'a str,
        objective_type: VarInt,
    },
    Remove,
    Update {
        objective_value: &'a str,
        objective_type: VarInt,
    },
}
