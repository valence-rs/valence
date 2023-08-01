use super::*;

#[derive(Clone, Debug, Encode, Decode, Packet)]
#[packet(id = packet_id::OPEN_SCREEN_S2C)]
pub struct OpenScreenS2c<'a> {
    pub window_id: VarInt,
    pub window_type: WindowType,
    pub window_title: Cow<'a, Text>,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum WindowType {
    Generic9x1,
    Generic9x2,
    Generic9x3,
    Generic9x4,
    Generic9x5,
    Generic9x6,
    Generic3x3,
    Anvil,
    Beacon,
    BlastFurnace,
    BrewingStand,
    Crafting,
    Enchantment,
    Furnace,
    Grindstone,
    Hopper,
    Lectern,
    Loom,
    Merchant,
    ShulkerBox,
    Smithing,
    Smoker,
    Cartography,
    Stonecutter,
}
