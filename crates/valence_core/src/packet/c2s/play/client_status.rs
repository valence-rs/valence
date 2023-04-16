use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
pub enum ClientStatusC2s {
    PerformRespawn,
    RequestStats,
}
