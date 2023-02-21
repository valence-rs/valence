use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x49]
pub struct UpdateSelectedSlotS2c {
    pub slot: u8,
}
