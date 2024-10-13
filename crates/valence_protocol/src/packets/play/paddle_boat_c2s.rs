use crate::{Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct PaddleBoatC2s {
    pub left_paddle_turning: bool,
    pub right_paddle_turning: bool,
}
