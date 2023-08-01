use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAY_PING_S2C)]
pub struct PlayPingS2c {
    pub id: i32,
}
