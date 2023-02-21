use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x0b]
pub struct CloseHandledScreenC2s {
    pub window_id: i8,
}
