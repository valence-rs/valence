use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::SCREEN_HANDLER_SLOT_UPDATE_S2C)]
pub struct ScreenHandlerSlotUpdateS2c<'a> {
    pub window_id: i8,
    pub state_id: VarInt,
    pub slot_idx: i16,
    pub slot_data: Cow<'a, Option<ItemStack>>,
}
