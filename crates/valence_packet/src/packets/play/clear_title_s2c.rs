use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CLEAR_TITLE_S2C)]
pub struct ClearTitleS2c {
    pub reset: bool,
}
