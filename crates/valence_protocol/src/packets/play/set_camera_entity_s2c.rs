use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct SetCameraEntityS2c {
    pub entity_id: VarInt,
}
