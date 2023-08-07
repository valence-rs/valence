use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::BUTTON_CLICK_C2S)]
pub struct ButtonClickC2s {
    pub window_id: i8,
    pub button_id: i8,
}
