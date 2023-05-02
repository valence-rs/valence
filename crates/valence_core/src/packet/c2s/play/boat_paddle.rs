use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct BoatPaddleStateC2s {
    pub left_paddle_turning: bool,
    pub right_paddle_turning: bool,
}
