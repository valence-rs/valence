use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x18]
pub struct BoatPaddleStateC2s {
    pub left_paddle_turning: bool,
    pub right_paddle_turning: bool,
}
