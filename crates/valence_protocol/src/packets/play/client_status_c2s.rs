use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub enum ClientStatusC2s {
    PerformRespawn,
    RequestStats,
}
