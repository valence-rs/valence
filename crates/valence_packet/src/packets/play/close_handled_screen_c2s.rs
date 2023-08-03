use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CLOSE_HANDLED_SCREEN_C2S)]
pub struct CloseHandledScreenC2s {
    pub window_id: i8,
}
