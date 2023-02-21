use crate::item::ItemStack;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x2b]
pub struct CreativeInventoryActionC2s {
    pub slot: i16,
    pub clicked_item: Option<ItemStack>,
}
