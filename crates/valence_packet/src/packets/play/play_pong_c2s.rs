use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::PLAY_PONG_C2S)]
pub struct PlayPongC2s {
    pub id: i32,
}
