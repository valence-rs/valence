use valence_derive::{Decode, Encode};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Encode, Decode)]
pub enum InventoryKind {
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

impl InventoryKind {
    /// The number of slots in this inventory, not counting the player's main
    /// inventory slots.
    pub const fn slot_count(self) -> usize {
        match self {
            InventoryKind::Generic9x1 => 9 * 1,
            InventoryKind::Generic9x2 => 9 * 2,
            InventoryKind::Generic9x3 => 9 * 3,
            InventoryKind::Generic9x4 => 9 * 4,
            InventoryKind::Generic9x5 => 9 * 5,
            InventoryKind::Generic9x6 => 9 * 6,
            InventoryKind::Generic3x3 => 3 * 3,
            InventoryKind::Anvil => 4,
            InventoryKind::Beacon => 1,
            InventoryKind::BlastFurnace => 3,
            InventoryKind::BrewingStand => 5,
            InventoryKind::Crafting => 10,
            InventoryKind::Enchantment => 2,
            InventoryKind::Furnace => 3,
            InventoryKind::Grindstone => 3,
            InventoryKind::Hopper => 5,
            InventoryKind::Lectern => 1,
            InventoryKind::Loom => 4,
            InventoryKind::Merchant => 3,
            InventoryKind::ShulkerBox => 27,
            InventoryKind::Smithing => 3,
            InventoryKind::Smoker => 3,
            InventoryKind::Cartography => 3,
            InventoryKind::Stonecutter => 2,
        }
    }
}
