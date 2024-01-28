use crate::{Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(name = "COMMON_PING_S2C")]
pub struct PlayPingS2c {
    pub id: i32,
}
