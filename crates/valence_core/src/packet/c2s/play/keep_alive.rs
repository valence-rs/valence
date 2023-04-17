use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct KeepAliveC2s {
    pub id: u64,
}
