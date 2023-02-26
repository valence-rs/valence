use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct EntityVelocityUpdateS2c {
    pub entity_id: VarInt,
    pub velocity: [i16; 3],
}
