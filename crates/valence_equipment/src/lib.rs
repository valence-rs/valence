use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_server::client::{Client, FlushPacketsSet, SpawnClientsSet};
use valence_server::entity::EntityId;
use valence_server::protocol::packets::play::entity_equipment_update_s2c::EquipmentEntry;
use valence_server::protocol::packets::play::EntityEquipmentUpdateS2c;
use valence_server::protocol::WritePacket;
use valence_server::ItemStack;

pub struct EquipmentPlugin;

impl Plugin for EquipmentPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, init_new_client_equipment.after(SpawnClientsSet))
            .add_systems(
                PostUpdate,
                (emit_equipment_change_event, update_equipment).before(FlushPacketsSet),
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
}

fn init_new_client_equipment(clients: Query<Entity, Added<Client>>, mut commands: Commands) {
    for entity in &clients {
        commands.entity(entity).insert(Equipment::default());
    }
}

#[derive(Debug, Clone, Event)]
pub struct EquipmentChangeEvent {
    pub client: Entity,
}

fn emit_equipment_change_event(
    mut clients: Query<(Entity, &mut Equipment), Changed<Equipment>>,
    mut event_writer: EventWriter<EquipmentChangeEvent>,
) {
    for (entity, mut equipment) in &mut clients {
        if equipment.changed != 0 {
            event_writer.send(EquipmentChangeEvent { client: entity });

            equipment.changed = 0;
        }
    }
}

fn update_equipment(
    mut clients: Query<(&EntityId, Option<&mut Client>, &Equipment)>,
    mut events: EventReader<EquipmentChangeEvent>,
) {
    for event in events.read() {
        let Ok((entity_id, _, equipment)) = clients.get(event.client) else {
            continue;
        };

        // The entity ID of the entity that changed equipment.
        let entity_id_changed_equipment = entity_id.get();

        let mut entries = Vec::with_capacity(Equipment::SLOT_COUNT);
        for slot in 0..Equipment::SLOT_COUNT {
            let item = equipment.slot(slot as u8);
            entries.push(EquipmentEntry {
                slot: slot as i8,
                item: item.clone(),
            });
        }

        for (entity_id, client, _) in &mut clients {
            // Dont send the packet to the entity that changed equipment.
            if entity_id.get() == entity_id_changed_equipment {
                continue;
            }

            if let Some(mut client) = client {
                client.write_packet(&EntityEquipmentUpdateS2c {
                    entity_id: entity_id_changed_equipment.into(),
                    equipment: entries.clone(),
                });
            }
        }
    }
}
