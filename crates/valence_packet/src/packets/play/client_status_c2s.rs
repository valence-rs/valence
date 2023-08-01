use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CLIENT_STATUS_C2S)]
pub enum ClientStatusC2s {
    PerformRespawn,
    RequestStats,
}
