use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct PlayPingS2c {
    pub id: i32,
}
