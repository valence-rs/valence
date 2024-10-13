use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_server::client::{Client, FlushPacketsSet, LoadEntityForClientEvent, SpawnClientsSet};
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
                init_new_client_equipment.after(SpawnClientsSet),
                on_entity_init,
            ),
        )
        .add_systems(
            Update,
            (
                update_equipment.before(FlushPacketsSet),
                emit_equipment_change_event,
                on_entity_load.before(FlushPacketsSet),
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
    pub const BOOTS_IDX: u8 = 2;
    pub const LEGGINGS_IDX: u8 = 3;
    pub const CHESTPLATE_IDX: u8 = 4;
    pub const HELMET_IDX: u8 = 5;

    pub fn new(equipment: [ItemStack; Self::SLOT_COUNT]) -> Self {
        Self {
            equipment,
            changed: u8::MAX,
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

    pub fn boots(&self) -> &ItemStack {
        self.slot(Self::BOOTS_IDX)
    }

    pub fn leggings(&self) -> &ItemStack {
        self.slot(Self::LEGGINGS_IDX)
    }

    pub fn chestplate(&self) -> &ItemStack {
        self.slot(Self::CHESTPLATE_IDX)
    }

    pub fn helmet(&self) -> &ItemStack {
        self.slot(Self::HELMET_IDX)
    }

    pub fn set_main_hand(&mut self, item: ItemStack) {
        self.set_slot(Self::MAIN_HAND_IDX, item);
    }

    pub fn set_off_hand(&mut self, item: ItemStack) {
        self.set_slot(Self::OFF_HAND_IDX, item);
    }

    pub fn set_boots(&mut self, item: ItemStack) {
        self.set_slot(Self::BOOTS_IDX, item);
    }

    pub fn set_leggings(&mut self, item: ItemStack) {
        self.set_slot(Self::LEGGINGS_IDX, item);
    }

    pub fn set_chestplate(&mut self, item: ItemStack) {
        self.set_slot(Self::CHESTPLATE_IDX, item);
    }

    pub fn set_helmet(&mut self, item: ItemStack) {
        self.set_slot(Self::HELMET_IDX, item);
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

fn init_new_client_equipment(clients: Query<Entity, Added<Client>>, mut commands: Commands) {
    for entity in &clients {
        commands.entity(entity).insert(Equipment::default());
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

fn emit_equipment_change_event(
    mut clients: Query<(Entity, &mut Equipment), Changed<Equipment>>,
    mut event_writer: EventWriter<EquipmentChangeEvent>,
) {
    for (entity, mut equipment) in &mut clients {
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

            event_writer.send(EquipmentChangeEvent {
                client: entity,
                changed: slots_changed,
            });

            equipment.changed = 0;
        }
    }
}

fn update_equipment(
    mut clients: Query<(&EntityId, &EntityLayerId, &Position)>,
    mut entity_layer_query: Query<&mut EntityLayer>,
    mut events: EventReader<EquipmentChangeEvent>,
) {
    for event in events.read() {
        let Ok((entity_id, entity_layer_id, position)) = clients.get(event.client) else {
            continue;
        };

        let Ok(mut entity_layer) = entity_layer_query.get_mut(entity_layer_id.0) else {
            continue;
        };

        // The entity ID of the entity that changed equipment.
        let entity_id_changed_equipment = entity_id.get();
        let entity_pos_changed_equipment = *position;

        let mut entries: Vec<EquipmentEntry> = Vec::with_capacity(event.changed.len());
        for change in &event.changed {
            entries.push(EquipmentEntry {
                slot: change.idx as i8,
                item: change.stack.clone(),
            });
        }

        for (entity_id, _, _) in &mut clients {
            // Dont send the packet to the entity that changed equipment.
            if entity_id.get() == entity_id_changed_equipment {
                continue;
            }

            entity_layer
                .view_writer(entity_pos_changed_equipment.0)
                .write_packet(&EntityEquipmentUpdateS2c {
                    entity_id: entity_id_changed_equipment.into(),
                    equipment: entries.clone(),
                })
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
