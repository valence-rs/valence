use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x2e]
pub struct PlayPingS2c {
    pub id: i32,
}
