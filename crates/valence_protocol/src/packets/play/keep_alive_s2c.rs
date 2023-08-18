use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct KeepAliveS2c {
    pub id: u64,
}
