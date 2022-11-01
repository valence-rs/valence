use std::ops::Range;

use crate::item::ItemStack;
use crate::protocol::{SlotId, VarInt};
use crate::slab_versioned::{Key, VersionedSlab};

pub trait Inventory {
    fn slot(&self, slot_id: SlotId) -> Option<&ItemStack>;
    /// Sets the slot to the desired contents. Returns the previous contents of
    /// the slot.
    fn set_slot(&mut self, slot_id: SlotId, slot: Option<ItemStack>) -> Option<ItemStack>;
    fn slot_range(&self) -> Range<SlotId>;

    fn slot_count(&self) -> usize {
        self.slot_range().count()
    }

    // TODO: `entry()` style api

    fn slots(&self) -> Vec<Option<&ItemStack>> {
        (0..self.slot_count())
            .map(|s| self.slot(s as SlotId))
            .collect()
    }

    /// Decreases the count for stack in the slot by amount. If there is not
    /// enough items in the stack to perform the operation, then it will fail.
    ///
    /// Returns `Ok` if the stack had enough items, and the operation was
    /// carried out. Otherwise, it returns `Err` if `amount > stack.count()`,
    /// and no changes were made to the inventory.
    fn consume(&mut self, slot_id: SlotId, amount: impl Into<u8>) -> Result<(), ()> {
        let amount: u8 = amount.into();
        let slot = self.slot(slot_id).cloned();
        if slot.is_some() {
            // Intentionally not using `if let` so stack can be moved out of the slot to
            // avoid another clone later.
            let mut stack = slot.unwrap();
            if amount > stack.count() {
                return Err(());
            }
            let slot = if amount == stack.count() {
                None
            } else {
                stack.set_count(stack.count() - amount);
                Some(stack)
            };

            self.set_slot(slot_id, slot);
        }
        Ok(())
    }
}

pub(crate) trait InventoryDirtyable {
    fn mark_dirty(&mut self, dirty: bool);
    fn is_dirty(&self) -> bool;
}

/// Represents a player's Inventory.
#[derive(Debug, Clone)]
pub struct PlayerInventory {
    pub(crate) slots: Box<[Option<ItemStack>; 46]>,
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

    pub(crate) fn new() -> Self {
        Self {
            // Can't do the shorthand because Option<ItemStack> is not Copy.
            slots: Box::new(std::array::from_fn(|_| None)),
            dirty: true,
            state_id: Default::default(),
        }
    }
}

impl Inventory for PlayerInventory {
    fn slot(&self, slot_id: SlotId) -> Option<&ItemStack> {
        if !self.slot_range().contains(&slot_id) {
            return None;
        }
        self.slots[slot_id as usize].as_ref()
    }

    fn set_slot(&mut self, slot_id: SlotId, slot: Option<ItemStack>) -> Option<ItemStack> {
        if !self.slot_range().contains(&slot_id) {
            return None;
        }
        self.mark_dirty(true);
        std::mem::replace(&mut self.slots[slot_id as usize], slot)
    }

    fn slot_range(&self) -> Range<SlotId> {
        0..(self.slots.len() as SlotId)
    }
}

impl InventoryDirtyable for PlayerInventory {
    fn mark_dirty(&mut self, dirty: bool) {
        self.dirty = dirty
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }
}

#[derive(Debug, Clone)]
pub struct ConfigurableInventory {
    slots: Vec<Option<ItemStack>>,
    /// The slots that the player can place items into for crafting. The
    /// crafting result slot is always zero, and should not be included in this
    /// range.
    #[allow(dead_code)] // TODO: implement crafting
    crafting_slots: Option<Range<SlotId>>,
    /// The type of window that should be used to display this inventory.
    pub window_type: VarInt,
    dirty: bool,
}

impl ConfigurableInventory {
    pub fn new(size: usize, window_type: VarInt, crafting_slots: Option<Range<SlotId>>) -> Self {
        ConfigurableInventory {
            slots: vec![None; size],
            crafting_slots,
            window_type,
            dirty: false,
        }
    }
}

impl Inventory for ConfigurableInventory {
    fn slot(&self, slot_id: SlotId) -> Option<&ItemStack> {
        if !self.slot_range().contains(&slot_id) {
            return None;
        }
        self.slots[slot_id as usize].as_ref()
    }

    fn set_slot(&mut self, slot_id: SlotId, slot: Option<ItemStack>) -> Option<ItemStack> {
        if !self.slot_range().contains(&slot_id) {
            return None;
        }
        self.mark_dirty(true);
        std::mem::replace(&mut self.slots[slot_id as usize], slot)
    }

    fn slot_range(&self) -> Range<SlotId> {
        0..(self.slots.len() as SlotId)
    }
}

impl InventoryDirtyable for ConfigurableInventory {
    fn mark_dirty(&mut self, dirty: bool) {
        self.dirty = dirty
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }
}
/// Represents what the player sees when they open an object's Inventory.
///
/// This exists because when an object inventory screen is being shown to the
/// player, it also shows part of the player's inventory so they can move items
/// between the inventories.
pub struct WindowInventory {
    pub window_id: u8,
    pub object_inventory: InventoryId,
}

impl WindowInventory {
    pub fn new(window_id: impl Into<u8>, object_inventory: InventoryId) -> Self {
        WindowInventory {
            window_id: window_id.into(),
            object_inventory,
        }
    }

    pub fn slots<'a>(
        &self,
        obj_inventory: &'a ConfigurableInventory,
        player_inventory: &'a PlayerInventory,
    ) -> Vec<Option<&'a ItemStack>> {
        let total_slots = obj_inventory.slots.len() + PlayerInventory::GENERAL_SLOTS.len();
        (0..total_slots)
            .map(|s| {
                if s < obj_inventory.slot_count() {
                    return obj_inventory.slot(s as SlotId);
                }
                let offset = obj_inventory.slot_count();
                player_inventory.slot((s - offset) as SlotId + PlayerInventory::GENERAL_SLOTS.start)
            })
            .collect()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct InventoryId(Key);

/// Manages all inventories that are present in the server.
pub struct Inventories {
    slab: VersionedSlab<ConfigurableInventory>,
}

impl Inventories {
    pub(crate) fn new() -> Self {
        Self {
            slab: VersionedSlab::new(),
        }
    }

    /// Creates a new inventory on a server.
    pub fn insert(
        &mut self,
        inv: ConfigurableInventory,
    ) -> (InventoryId, &mut ConfigurableInventory) {
        let (key, value) = self.slab.insert(inv);
        (InventoryId(key), value)
    }

    /// Removes an inventory from the server.
    pub fn remove(&mut self, inv: InventoryId) -> Option<ConfigurableInventory> {
        self.slab.remove(inv.0)
    }

    /// Returns the number of inventories in this container.
    pub fn len(&self) -> usize {
        self.slab.len()
    }

    /// Returns `true` if there are no inventories.
    pub fn is_empty(&self) -> bool {
        self.slab.len() == 0
    }

    pub fn get(&self, inv: InventoryId) -> Option<&ConfigurableInventory> {
        self.slab.get(inv.0)
    }

    pub fn get_mut(&mut self, inv: InventoryId) -> Option<&mut ConfigurableInventory> {
        self.slab.get_mut(inv.0)
    }

    pub(crate) fn update(&mut self) {
        // now that we have synced all the dirty inventories, mark them as clean
        for (_, inv) in self.slab.iter_mut() {
            inv.mark_dirty(false);
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::item::{ItemKind, ItemStack};

    #[test]
    fn test_get_set_slots() {
        let mut inv = PlayerInventory::new();
        let slot = Some(ItemStack::new(ItemKind::Bone, 12, None));
        let prev = inv.set_slot(9, slot.clone());
        assert_eq!(inv.slot(9), slot.as_ref());
        assert_eq!(prev, None);
    }

    #[test]
    fn test_consume() {
        let mut inv = PlayerInventory::new();
        let slot_id = 9;
        let slot = Some(ItemStack::new(ItemKind::Bone, 12, None));
        inv.set_slot(slot_id, slot.clone());
        assert!(matches!(inv.consume(slot_id, 2), Ok(_)));
        assert_eq!(inv.slot(slot_id).unwrap().count(), 10);
        assert!(matches!(inv.consume(slot_id, 20), Err(_)));
        assert_eq!(inv.slot(slot_id).unwrap().count(), 10);
        assert!(matches!(inv.consume(slot_id, 10), Ok(_)));
        assert_eq!(inv.slot(slot_id), None);
    }
}
