use valence_inventory::player_inventory::PlayerInventory;
use valence_inventory::{HeldItem, Inventory};
use valence_server::entity::player::PlayerEntity;

use super::*;
#[derive(Debug, Default, Clone, Component)]
pub struct EquipmentInventorySync;

/// Syncs the player [`Equipment`] with the [`Inventory`].
/// If a change in the player's inventory and in the equipment occurs in the
/// same tick, the equipment change has priority.
pub(crate) fn equipment_inventory_sync(
    mut clients: Query<
        (&mut Equipment, &mut Inventory, &mut HeldItem),
        (
            Or<(Changed<Equipment>, Changed<Inventory>, Changed<HeldItem>)>,
            With<EquipmentInventorySync>,
            With<PlayerEntity>,
        ),
    >,
) {
    for (mut equipment, mut inventory, held_item) in &mut clients {
        // Equipment change has priority over held item changes
        if equipment.changed & (1 << Equipment::MAIN_HAND_IDX) != 0 {
            let item = equipment.main_hand().clone();
            inventory.set_slot(held_item.slot(), item);
        } else if held_item.is_changed() {
            let item = inventory.slot(held_item.slot()).clone();
            equipment.set_main_hand(item);
        }

        let slots = [
            (Equipment::OFF_HAND_IDX, PlayerInventory::SLOT_OFFHAND),
            (Equipment::HEAD_IDX, PlayerInventory::SLOT_HEAD),
            (Equipment::CHEST_IDX, PlayerInventory::SLOT_CHEST),
            (Equipment::LEGS_IDX, PlayerInventory::SLOT_LEGS),
            (Equipment::FEET_IDX, PlayerInventory::SLOT_FEET),
        ];

        for (equipment_slot, inventory_slot) in slots {
            // Equipment has priority over inventory changes
            if equipment.changed & (1 << equipment_slot) != 0 {
                let item = equipment.slot(equipment_slot).clone();
                inventory.set_slot(inventory_slot, item);
            } else if inventory.changed & (1 << inventory_slot) != 0 {
                let item = inventory.slot(inventory_slot).clone();
                equipment.set_slot(equipment_slot, item);
            }
        }
    }
}

pub(crate) fn on_attach_inventory_sync(
    entities: Query<Option<&PlayerEntity>, (Added<EquipmentInventorySync>, With<Inventory>)>,
) {
    for entity in &entities {
        if entity.is_none() {
            tracing::warn!(
                "EquipmentInventorySync attached to non-player entity, this will have no effect"
            );
        }
    }
}
