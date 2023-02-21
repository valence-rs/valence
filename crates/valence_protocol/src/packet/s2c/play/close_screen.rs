use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x0f]
pub struct CloseScreenS2c {
    /// Ignored by notchian clients.
    pub window_id: u8,
}
