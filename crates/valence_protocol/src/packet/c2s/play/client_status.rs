use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x06]
pub enum ClientStatusC2s {
    PerformRespawn,
    RequestStats,
}
