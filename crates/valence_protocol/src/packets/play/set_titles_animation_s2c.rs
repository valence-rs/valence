use crate::{Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct SetTitlesAnimationS2c {
    pub fade_in: i32,
    pub stay: i32,
    pub fade_out: i32,
}
