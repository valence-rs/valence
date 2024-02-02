use crate::{Decode, Encode, Packet, VarInt};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct SetCameraEntityS2c {
    pub entity_id: VarInt,
}
