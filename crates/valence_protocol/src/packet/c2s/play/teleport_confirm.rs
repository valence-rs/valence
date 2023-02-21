use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x00]
pub struct TeleportConfirmC2s {
    pub teleport_id: VarInt,
}
