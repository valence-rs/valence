use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::TITLE_FADE_S2C)]
pub struct TitleFadeS2c {
    pub fade_in: i32,
    pub stay: i32,
    pub fade_out: i32,
}
