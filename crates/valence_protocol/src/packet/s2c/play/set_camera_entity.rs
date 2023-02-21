use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x48]
pub struct SetCameraEntityS2c {
    pub entity_id: VarInt,
}
