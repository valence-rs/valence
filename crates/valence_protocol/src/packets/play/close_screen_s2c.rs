use crate::{Decode, Encode, Packet};

#[derive(Copy, Clone, Debug, Encode, Decode, Packet)]
pub struct CloseScreenS2c {
    /// Ignored by notchian clients.
    pub window_id: u8,
}
