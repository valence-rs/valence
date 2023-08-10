use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::KEEP_ALIVE_C2S)]
pub struct KeepAliveC2s {
    pub id: u64,
}
