use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct ClearTitleS2c {
    pub reset: bool,
}
