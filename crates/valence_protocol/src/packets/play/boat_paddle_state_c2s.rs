use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct BoatPaddleStateC2s {
    pub left_paddle_turning: bool,
    pub right_paddle_turning: bool,
}
