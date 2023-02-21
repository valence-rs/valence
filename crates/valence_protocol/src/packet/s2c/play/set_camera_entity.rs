use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub struct SetCameraEntityS2c {
    pub entity_id: VarInt,
}
