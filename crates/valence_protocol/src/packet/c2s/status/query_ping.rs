use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct QueryPingC2s {
    pub payload: u64,
}
