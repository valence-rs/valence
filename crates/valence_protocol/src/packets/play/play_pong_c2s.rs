use crate::{Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(name = "COMMON_PONG_C2S")]
pub struct PlayPongC2s {
    pub id: i32,
}
