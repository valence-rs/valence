use std::ops::RangeInclusive;

use derive_more::{Deref, DerefMut};

use crate::{into_inventory::IntoInventory, Inventory};

pub struct PlayerInventory(Inventory);

impl PlayerInventory {
    pub const SLOT_OFFHAND: u16 = 45;
    pub const SLOT_HEAD: u16 = 5;
    pub const SLOT_CHEST: u16 = 6;
    pub const SLOT_LEGS: u16 = 7;
    pub const SLOT_FEET: u16 = 8;
    pub const SLOTS_CRAFT_INPUT: RangeInclusive<u16> = 1..=4;
    pub const SLOT_CRAFT_RESULT: u16 = 0;
    pub const SLOTS_HOTBAR: RangeInclusive<u16> = 36..=44;
    pub const SLOTS_MAIN: RangeInclusive<u16> = 9..=44;

    pub const fn hotbar_to_slot(hotbar: u8) -> u16 {
        *Self::SLOTS_HOTBAR.start() + (hotbar as u16)
    }

    pub const fn slot_to_hotbar(slot: u16) -> u8 {
        (slot - *Self::SLOTS_HOTBAR.start()) as u8
    }
}

impl IntoInventory for PlayerInventory {
    fn into_inventory(self) -> Inventory {
        self.0
    }
}

impl Deref for PlayerInventory {
    type Target = Inventory;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for PlayerInventory {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
