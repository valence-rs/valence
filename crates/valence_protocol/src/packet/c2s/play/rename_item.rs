use crate::{Decode, Encode};

#[derive(Copy, Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x23]
pub struct RenameItemC2s<'a> {
    pub item_name: &'a str,
}
