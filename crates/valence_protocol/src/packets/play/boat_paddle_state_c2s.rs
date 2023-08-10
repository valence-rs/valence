use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::BOAT_PADDLE_STATE_C2S)]
pub struct BoatPaddleStateC2s {
    pub left_paddle_turning: bool,
    pub right_paddle_turning: bool,
}
