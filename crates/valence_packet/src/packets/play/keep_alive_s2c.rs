use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::KEEP_ALIVE_S2C)]
pub struct KeepAliveS2c {
    pub id: u64,
}
