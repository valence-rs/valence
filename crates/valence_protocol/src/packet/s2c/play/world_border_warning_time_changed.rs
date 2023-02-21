use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x46]
pub struct WorldBorderWarningTimeChangedS2c {
    pub warning_time: VarInt,
}
