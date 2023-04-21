use crate::packet::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, Decode)]
pub struct ButtonClickC2s {
    pub window_id: i8,
    pub button_id: i8,
}
