use crate::packet::var_int::VarInt;
use crate::packet::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct SetCameraEntityS2c {
    pub entity_id: VarInt,
}
