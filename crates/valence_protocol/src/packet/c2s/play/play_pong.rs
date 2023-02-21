use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x1f]
pub struct PlayPongC2s {
    pub id: i32,
}
