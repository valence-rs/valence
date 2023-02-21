use std::borrow::Cow;

use crate::item::ItemStack;
use crate::var_int::VarInt;
use crate::{Decode, Encode};

#[derive(Clone, Debug, Encode, EncodePacket, Decode, DecodePacket)]
#[packet_id = 0x12]
pub struct ScreenHandlerSlotUpdateS2c<'a> {
    pub window_id: i8,
    pub state_id: VarInt,
    pub slot_idx: i16,
    pub slot_data: Cow<'a, Option<ItemStack>>,
}
