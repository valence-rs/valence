use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
pub struct InventoryS2c<'a> {
    pub window_id: u8,
    pub state_id: VarInt,
    pub slots: Cow<'a, [Option<ItemStack>]>,
    pub carried_item: Cow<'a, Option<ItemStack>>,
}
