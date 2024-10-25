use crate::{Decode, Encode, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct ProjectilePowerS2c {
    pub entity_id: VarInt,
    pub power: f64,
}
