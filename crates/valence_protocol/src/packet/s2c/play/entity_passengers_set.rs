use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct EntityPassengersSetS2c {
    /// Vehicle's entity id
    pub entity_id: VarInt,
    pub passengers: Vec<VarInt>,
}
