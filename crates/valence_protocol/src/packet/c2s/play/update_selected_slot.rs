use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x28]
pub struct UpdateSelectedSlotC2s {
    pub slot: i16,
}
