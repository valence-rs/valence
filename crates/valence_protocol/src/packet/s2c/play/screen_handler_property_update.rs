use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x11]
pub struct ScreenHandlerPropertyUpdateS2c {
    pub window_id: u8,
    pub property: i16,
    pub value: i16,
}
