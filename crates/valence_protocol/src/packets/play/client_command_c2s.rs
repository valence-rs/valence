use crate::{Decode, Encode, Packet};

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub enum ClientCommandC2s {
    PerformRespawn,
    RequestStats,
}
