use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct CloseScreenS2c {
    /// Ignored by notchian clients.
    pub window_id: u8,
}
