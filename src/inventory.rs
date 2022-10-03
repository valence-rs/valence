use std::ops::Range;
use std::sync::{Arc, Mutex};

use crate::protocol::{Slot, SlotId};

pub trait Inventory {
    fn get_slot(&self, slot_id: SlotId) -> Slot;
    fn set_slot(&mut self, slot_id: SlotId, slot: Slot);
    fn slot_count(&self) -> usize;
    fn is_dirty(&self) -> bool;

    // TODO: `entry()` style api

    fn slots(&self) -> Vec<Slot> {
        (0..self.slot_count())
            .map(|s| self.get_slot(s as SlotId))
            .collect()
    }
}

pub trait InventoryDirtyable {
    fn mark_dirty(&mut self, dirty: bool);
}

pub trait CraftingInventory {
    fn craft_result_slot() -> SlotId;
    fn craft_table_slots() -> Range<SlotId>;
}

/// Represents a player's Inventory.
#[derive(Debug, Clone)]
pub struct PlayerInventory {
    pub(crate) slots: Box<[Slot; 46]>,
    dirty: bool,
    pub(crate) state_id: i32,
}

impl PlayerInventory {
    /// General slots are the slots that can hold all items, including the
    /// hotbar, excluding offhand. These slots are shown when the player is
    /// looking at another inventory.
    pub const GENERAL_SLOTS: Range<SlotId> = 9..45;
    pub const HOTBAR_SLOTS: Range<SlotId> = 36..45;

    pub fn hotbar_to_slot(hotbar_slot: i16) -> Option<SlotId> {
        if !(0..=8).contains(&hotbar_slot) {
            return None;
        }

        Some(Self::HOTBAR_SLOTS.start + hotbar_slot)
    }
}

impl Default for PlayerInventory {
    fn default() -> Self {
        Self {
            // Can't do the shorthand because Slot is not Copy.
            slots: Box::new(std::array::from_fn(|_| Slot::Empty)),
            dirty: true,
            state_id: Default::default(),
        }
    }
}

impl Inventory for PlayerInventory {
    fn get_slot(&self, slot_id: SlotId) -> Slot {
        if slot_id < 0 || slot_id >= self.slot_count() as i16 {
            // TODO: dont panic
            panic!("invalid slot id")
        }
        self.slots[slot_id as usize].clone()
    }

    fn set_slot(&mut self, slot_id: SlotId, slot: Slot) {
        if slot_id < 0 || slot_id >= self.slot_count() as i16 {
            // TODO: dont panic
            panic!("invalid slot id")
        }
        self.slots[slot_id as usize] = slot;
        self.mark_dirty(true);
    }

    fn slot_count(&self) -> usize {
        self.slots.len()
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }
}

impl InventoryDirtyable for PlayerInventory {
    fn mark_dirty(&mut self, dirty: bool) {
        self.dirty = dirty
    }
}

#[derive(Debug, Clone)]
pub struct ConfigurableInventory {
    slots: Vec<Slot>,
    /// The slots that the player can place items into for crafting. The
    /// crafting result slot is always zero, and should not be included in this
    /// range.
    crafting_slots: Option<Range<SlotId>>,
    dirty: bool,
}

impl ConfigurableInventory {
    pub fn new(size: usize, crafting_slots: Option<Range<SlotId>>) -> Self {
        ConfigurableInventory {
            slots: vec![Slot::Empty; size],
            crafting_slots,
            dirty: false,
        }
    }
}

impl Inventory for ConfigurableInventory {
    fn get_slot(&self, slot_id: SlotId) -> Slot {
        if slot_id < 0 || slot_id >= self.slot_count() as i16 {
            // TODO: dont panic
            panic!("invalid slot id")
        }
        self.slots[slot_id as usize].clone()
    }

    fn set_slot(&mut self, slot_id: SlotId, slot: Slot) {
        if slot_id < 0 || slot_id >= self.slot_count() as i16 {
            // TODO: dont panic
            panic!("invalid slot id")
        }
        self.slots[slot_id as usize] = slot;
        self.mark_dirty(true);
    }

    fn slot_count(&self) -> usize {
        self.slots.len()
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }
}

impl InventoryDirtyable for ConfigurableInventory {
    fn mark_dirty(&mut self, dirty: bool) {
        self.dirty = dirty
    }
}

/// Represents what the player sees when they open an object's Inventory.
///
/// This exists because when an object inventory screen is being shown to the
/// player, it also shows part of the player's inventory so they can move items
/// between the inventories.
pub struct WindowInventory {
    pub window_id: u8,
    object_inventory: Arc<Mutex<dyn Inventory + Send>>,
    player_inventory: Arc<Mutex<PlayerInventory>>,
}

impl WindowInventory {
    pub fn new(
        window_id: impl Into<u8>,
        object_inventory: Arc<Mutex<dyn Inventory + Send>>,
        player_inventory: Arc<Mutex<PlayerInventory>>,
    ) -> Self {
        WindowInventory {
            window_id: window_id.into(),
            object_inventory,
            player_inventory,
        }
    }

    fn is_in_object(&self, slot_id: SlotId) -> bool {
        (slot_id as usize) < self.object_inventory.lock().unwrap().slot_count()
    }

    fn to_player_slot(&self, slot_id: SlotId) -> SlotId {
        let first_general_slot = PlayerInventory::GENERAL_SLOTS.start;
        slot_id - self.object_inventory.lock().unwrap().slot_count() as SlotId + first_general_slot
    }
}

impl Inventory for WindowInventory {
    fn get_slot(&self, slot_id: SlotId) -> Slot {
        if slot_id < 0 {
            // TODO: dont panic
            panic!("invalid slot id")
        }

        if self.is_in_object(slot_id) {
            self.object_inventory.lock().unwrap().get_slot(slot_id)
        } else {
            self.player_inventory
                .lock()
                .unwrap()
                .get_slot(self.to_player_slot(slot_id))
        }
    }

    fn set_slot(&mut self, slot_id: SlotId, slot: Slot) {
        if slot_id < 0 {
            // TODO: dont panic
            panic!("invalid slot id")
        }

        if self.is_in_object(slot_id) {
            self.object_inventory
                .lock()
                .unwrap()
                .set_slot(slot_id, slot)
        } else {
            self.player_inventory
                .lock()
                .unwrap()
                .set_slot(self.to_player_slot(slot_id), slot)
        }
    }

    fn slot_count(&self) -> usize {
        self.object_inventory.lock().unwrap().slot_count() + PlayerInventory::GENERAL_SLOTS.len()
    }

    fn is_dirty(&self) -> bool {
        self.player_inventory.lock().unwrap().is_dirty()
            || self.object_inventory.lock().unwrap().is_dirty()
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
        assert_eq!(inv.get_slot(9), slot);
    }
}
