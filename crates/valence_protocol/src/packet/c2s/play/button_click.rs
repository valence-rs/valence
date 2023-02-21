use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x09]
pub struct ButtonClickC2s {
    pub window_id: i8,
    pub button_id: i8,
}
