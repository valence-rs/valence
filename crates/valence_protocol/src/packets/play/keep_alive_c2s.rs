use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct KeepAliveC2s {
    pub id: u64,
}
