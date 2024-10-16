use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
#[cfg(feature = "inventory")]
pub use inventory_sync::EquipmentInventorySync;
use valence_server::client::{Client, FlushPacketsSet, LoadEntityForClientEvent};
use valence_server::entity::living::LivingEntity;
use valence_server::entity::{EntityId, EntityLayerId, Position};
use valence_server::protocol::packets::play::entity_equipment_update_s2c::EquipmentEntry;
use valence_server::protocol::packets::play::EntityEquipmentUpdateS2c;
use valence_server::protocol::WritePacket;
use valence_server::{EntityLayer, ItemStack, Layer};

pub struct EquipmentPlugin;

impl Plugin for EquipmentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (
                on_entity_init,
                #[cfg(feature = "inventory")]
                inventory_sync::on_attach_inventory_sync,
            ),
        )
        .add_systems(
            Update,
            (
                update_equipment.before(FlushPacketsSet),
                on_entity_load.before(FlushPacketsSet),
                #[cfg(feature = "inventory")]
                inventory_sync::equipment_inventory_sync,
            ),
        )
        .add_event::<EquipmentChangeEvent>();
    }
}

#[derive(Debug, Default, Clone, Component)]
pub struct Equipment {
    equipment: [ItemStack; Self::SLOT_COUNT],
    /// Contains a set bit for each modified slot in `slots`.
    #[doc(hidden)]
    pub(crate) changed: u8,
}

impl Equipment {
    pub const SLOT_COUNT: usize = 6;

    pub const MAIN_HAND_IDX: u8 = 0;
    pub const OFF_HAND_IDX: u8 = 1;
    pub const FEET_IDX: u8 = 2;
    pub const LEGS_IDX: u8 = 3;
    pub const CHEST_IDX: u8 = 4;
    pub const HEAD_IDX: u8 = 5;

    pub fn new(
        main_hand: ItemStack,
        off_hand: ItemStack,
        boots: ItemStack,
        leggings: ItemStack,
        chestplate: ItemStack,
        helmet: ItemStack,
    ) -> Self {
        Self {
            equipment: [main_hand, off_hand, boots, leggings, chestplate, helmet],
            changed: 0,
        }
    }

    pub fn slot(&self, idx: u8) -> &ItemStack {
        &self.equipment[idx as usize]
    }

    pub fn set_slot(&mut self, idx: u8, item: ItemStack) {
        assert!(
            idx < Self::SLOT_COUNT as u8,
            "slot index of {idx} out of bounds"
        );
        if self.equipment[idx as usize] != item {
            self.equipment[idx as usize] = item;
            self.changed |= 1 << idx;
        }
    }

    pub fn main_hand(&self) -> &ItemStack {
        self.slot(Self::MAIN_HAND_IDX)
    }

    pub fn off_hand(&self) -> &ItemStack {
        self.slot(Self::OFF_HAND_IDX)
    }

    pub fn feet(&self) -> &ItemStack {
        self.slot(Self::FEET_IDX)
    }

    pub fn legs(&self) -> &ItemStack {
        self.slot(Self::LEGS_IDX)
    }

    pub fn chest(&self) -> &ItemStack {
        self.slot(Self::CHEST_IDX)
    }

    pub fn head(&self) -> &ItemStack {
        self.slot(Self::HEAD_IDX)
    }

    pub fn set_main_hand(&mut self, item: ItemStack) {
        self.set_slot(Self::MAIN_HAND_IDX, item);
    }

    pub fn set_off_hand(&mut self, item: ItemStack) {
        self.set_slot(Self::OFF_HAND_IDX, item);
    }

    pub fn set_feet(&mut self, item: ItemStack) {
        self.set_slot(Self::FEET_IDX, item);
    }

    pub fn set_legs(&mut self, item: ItemStack) {
        self.set_slot(Self::LEGS_IDX, item);
    }

    pub fn set_chest(&mut self, item: ItemStack) {
        self.set_slot(Self::CHEST_IDX, item);
    }

    pub fn set_helmet(&mut self, item: ItemStack) {
        self.set_slot(Self::HEAD_IDX, item);
    }

    pub fn clear(&mut self) {
        for slot in 0..Self::SLOT_COUNT as u8 {
            self.set_slot(slot, ItemStack::EMPTY);
        }
    }

    pub fn is_default(&self) -> bool {
        self.equipment.iter().all(|item| item.is_empty())
    }
}

#[derive(Debug, Clone)]
pub struct EquipmentSlotChange {
    idx: u8,
    stack: ItemStack,
}

#[derive(Debug, Clone, Event)]
pub struct EquipmentChangeEvent {
    pub client: Entity,
    pub changed: Vec<EquipmentSlotChange>,
}

fn update_equipment(
    mut clients: Query<
        (Entity, &EntityId, &EntityLayerId, &Position, &mut Equipment),
        Changed<Equipment>,
    >,
    mut event_writer: EventWriter<EquipmentChangeEvent>,
    mut entity_layer: Query<&mut EntityLayer>,
) {
    for (entity, entity_id, entity_layer_id, position, mut equipment) in &mut clients {
        let Ok(mut entity_layer) = entity_layer.get_mut(entity_layer_id.0) else {
            continue;
        };

        if equipment.changed != 0 {
            let mut slots_changed: Vec<EquipmentSlotChange> =
                Vec::with_capacity(Equipment::SLOT_COUNT);

            for slot in 0..Equipment::SLOT_COUNT {
                if equipment.changed & (1 << slot) != 0 {
                    slots_changed.push(EquipmentSlotChange {
                        idx: slot as u8,
                        stack: equipment.equipment[slot].clone(),
                    });
                }
            }

            entity_layer
                .view_except_writer(position.0, entity)
                .write_packet(&EntityEquipmentUpdateS2c {
                    entity_id: entity_id.get().into(),
                    equipment: slots_changed
                        .iter()
                        .map(|change| EquipmentEntry {
                            slot: change.idx as i8,
                            item: change.stack.clone(),
                        })
                        .collect(),
                });

            event_writer.send(EquipmentChangeEvent {
                client: entity,
                changed: slots_changed,
            });

            equipment.changed = 0;
        }
    }
}

/// Gets called when the player loads an entity, for example
/// when the player gets in range of the entity.
fn on_entity_load(
    mut clients: Query<&mut Client>,
    entities: Query<(&EntityId, &Equipment)>,
    mut events: EventReader<LoadEntityForClientEvent>,
) {
    for event in events.read() {
        let Ok(mut client) = clients.get_mut(event.client) else {
            continue;
        };

        let Ok((entity_id, equipment)) = entities.get(event.entity_loaded) else {
            continue;
        };

        if equipment.is_default() {
            continue;
        }

        let mut entries: Vec<EquipmentEntry> = Vec::with_capacity(Equipment::SLOT_COUNT);
        for (idx, stack) in equipment.equipment.iter().enumerate() {
            entries.push(EquipmentEntry {
                slot: idx as i8,
                item: stack.clone(),
            });
        }

        client.write_packet(&EntityEquipmentUpdateS2c {
            entity_id: entity_id.get().into(),
            equipment: entries,
        });
    }
}

/// Add a default equipment component to all living entities when they are
/// initialized.
fn on_entity_init(
    mut commands: Commands,
    mut entities: Query<Entity, (Added<LivingEntity>, Without<Equipment>)>,
) {
    for entity in &mut entities {
        commands.entity(entity).insert(Equipment::default());
    }
}

#[cfg(feature = "inventory")]
mod inventory_sync {
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
                    "EquipmentInventorySync attached to non-player entity, this will have no \
                     effect"
                );
            }
        }
    }
}
