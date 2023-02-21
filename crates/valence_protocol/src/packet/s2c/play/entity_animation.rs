use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct EntityAnimationS2c {
    pub entity_id: VarInt,
    pub animation: u8, // TODO: use Animation enum.
}
