use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x47]
pub struct WorldBorderWarningBlocksChangedS2c {
    pub warning_blocks: VarInt,
}
