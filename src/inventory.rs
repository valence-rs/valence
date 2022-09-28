use std::ops::Range;

use crate::protocol::{Slot, SlotId};

pub trait Inventory {
    fn get_slot(&self, slot_id: SlotId) -> &Slot;
    fn set_slot(&mut self, slot_id: SlotId, slot: Slot);
    fn capacity(&self) -> usize;

    // TODO: `entry()` style api
}

pub trait CraftingInventory {
    fn craft_result_slot() -> SlotId;
    fn craft_table_slots() -> Range<SlotId>;
}

/// Represents a player's Inventory.
#[derive(Debug, Clone)]
pub struct PlayerInventory {
    pub(crate) slots: Box<[Slot; 46]>,
}

impl PlayerInventory {
    /// General slots are the slots that can hold all items, including the
    /// hotbar, excluding offhand. These slots are shown when the player is
    /// looking at another inventory.
    pub fn general_slots() -> Range<SlotId> {
        9..45
    }

    pub fn hotbar_slots() -> Range<SlotId> {
        36..45
    }

    pub fn hotbar_to_slot(hotbar_slot: i16) -> Option<SlotId> {
        if !(0..=8).contains(&hotbar_slot) {
            return None;
        }

        Some(Self::hotbar_slots().start + hotbar_slot)
    }
}

impl Default for PlayerInventory {
    fn default() -> Self {
        Self {
            // Can't do the shorthand because Slot is not Copy.
            slots: Box::new([
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
                Slot::Empty,
            ]),
        }
    }
}

impl Inventory for PlayerInventory {
    fn get_slot(&self, slot_id: SlotId) -> &Slot {
        if slot_id < 0 {
            // TODO: dont panic
            panic!("invalid slot id")
        }
        &self.slots[slot_id as usize]
    }

    fn set_slot(&mut self, slot_id: SlotId, slot: Slot) {
        if slot_id < 0 {
            // TODO: dont panic
            panic!("invalid slot id")
        }
        self.slots[slot_id as usize] = slot;
    }

    fn capacity(&self) -> usize {
        self.slots.len()
    }
}

/// Represents what the player sees when they open an object's Inventory.
///
/// This exists because when an object inventory screen is being shown to the
/// player, it also shows part of the player's inventory so they can move items
/// between the inventories.
#[derive(Debug)]
pub struct WindowInventory<T>
where
    T: Inventory,
{
    object_inventory: T,
    player_inventory: PlayerInventory,
}

impl<T: Inventory> WindowInventory<T> {
    fn is_in_object(&self, slot_id: SlotId) -> bool {
        (slot_id as usize) < self.object_inventory.capacity()
    }

    fn to_player_slot(&self, slot_id: SlotId) -> SlotId {
        let first_general_slot = PlayerInventory::general_slots().start;
        slot_id - self.object_inventory.capacity() as SlotId + first_general_slot
    }
}

impl<T: Inventory> Inventory for WindowInventory<T> {
    fn get_slot(&self, slot_id: SlotId) -> &Slot {
        if slot_id < 0 {
            // TODO: dont panic
            panic!("invalid slot id")
        }

        if self.is_in_object(slot_id) {
            self.object_inventory.get_slot(slot_id)
        } else {
            self.player_inventory.get_slot(self.to_player_slot(slot_id))
        }
    }

    fn set_slot(&mut self, slot_id: SlotId, slot: Slot) {
        if slot_id < 0 {
            // TODO: dont panic
            panic!("invalid slot id")
        }

        if self.is_in_object(slot_id) {
            self.object_inventory.set_slot(slot_id, slot)
        } else {
            self.player_inventory
                .set_slot(self.to_player_slot(slot_id), slot)
        }
    }

    fn capacity(&self) -> usize {
        self.object_inventory.capacity() + PlayerInventory::general_slots().len()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::itemstack::ItemStack;
    use crate::protocol::VarInt;

    #[test]
    fn test_get_set_slots() {
        let mut inv = PlayerInventory::default();
        let slot = Slot::Present(ItemStack {
            item_id: VarInt(7),
            item_count: 12,
            nbt: None,
        });
        inv.set_slot(9, slot.clone());
        assert_eq!(*inv.get_slot(9), slot);
    }
}
