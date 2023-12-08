use std::ops::RangeInclusive;

pub struct PlayerInventory;

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
    pub const MAIN_SIZE: u16 = *Self::SLOTS_MAIN.end() - *Self::SLOTS_MAIN.start() + 1;

    pub const fn hotbar_to_slot(hotbar: u8) -> u16 {
        *Self::SLOTS_HOTBAR.start() + (hotbar as u16)
    }

    pub const fn slot_to_hotbar(slot: u16) -> u8 {
        (slot - *Self::SLOTS_HOTBAR.start()) as u8
    }
}
