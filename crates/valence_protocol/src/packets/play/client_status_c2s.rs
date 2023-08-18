use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub enum ClientStatusC2s {
    PerformRespawn,
    RequestStats,
}
