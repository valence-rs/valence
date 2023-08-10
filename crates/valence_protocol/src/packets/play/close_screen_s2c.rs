use super::*;

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::CLOSE_SCREEN_S2C)]
pub struct CloseScreenS2c {
    /// Ignored by notchian clients.
    pub window_id: u8,
}
