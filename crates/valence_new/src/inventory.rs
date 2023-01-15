use std::iter::FusedIterator;
use std::num::Wrapping;

use bevy_ecs::prelude::*;
use tracing::warn;
use valence_protocol::packets::s2c::play::{
    CloseContainerS2c, OpenScreen, SetContainerContentEncode, SetContainerSlotEncode,
};
use valence_protocol::{InventoryKind, ItemStack, Text, VarInt, WindowType};

use crate::client::event::CloseContainer;
use crate::client::Client;

#[derive(Debug, Clone, Component)]
pub struct Inventory {
    title: Text,
    kind: InventoryKind,
    slots: Box<[Option<ItemStack>]>,
    /// Contains a set bit for each modified slot in `slots`.
    modified: u64,
    state_id: Wrapping<i32>,
}

impl Inventory {
    pub fn new(kind: InventoryKind) -> Self {
        // TODO: default title to the correct translation key instead
        Self::with_title(kind, "Inventory")
    }

    pub fn with_title(kind: InventoryKind, title: impl Into<Text>) -> Self {
        Inventory {
            title: title.into(),
            kind,
            slots: vec![None; kind.slot_count()].into(),
            modified: 0,
            state_id: Wrapping(0),
        }
    }

    pub fn slot(&self, idx: u16) -> Option<&ItemStack> {
        self.slots
            .get(idx as usize)
            .expect("slot index out of range")
            .as_ref()
    }

    pub fn replace_slot(
        &mut self,
        idx: u16,
        item: impl Into<Option<ItemStack>>,
    ) -> Option<ItemStack> {
        assert!(idx < self.slot_count(), "slot index out of range");

        let new = item.into();
        let old = &mut self.slots[idx as usize];

        if new != *old {
            self.modified |= 1 << idx;
        }

        std::mem::replace(old, new)
    }

    pub fn swap_slot(&mut self, idx_a: u16, idx_b: u16) {
        assert!(idx_a < self.slot_count(), "slot index out of range");
        assert!(idx_b < self.slot_count(), "slot index out of range");

        if idx_a == idx_b || self.slots[idx_a as usize] == self.slots[idx_b as usize] {
            // Nothing to do here, ignore.
            return;
        }

        self.modified |= 1 << idx_a;
        self.modified |= 1 << idx_b;

        self.slots.swap(idx_a as usize, idx_b as usize);
    }

    pub fn slot_count(&self) -> u16 {
        self.slots.len() as u16
    }

    pub fn slots(
        &self,
    ) -> impl ExactSizeIterator<Item = Option<&ItemStack>>
           + DoubleEndedIterator
           + FusedIterator
           + Clone
           + '_ {
        self.slots.iter().map(|item| item.as_ref())
    }

    pub fn kind(&self) -> InventoryKind {
        self.kind
    }

    pub fn title(&self) -> &Text {
        &self.title
    }

    pub fn replace_title(&mut self, title: impl Into<Text>) -> Text {
        // TODO: set title modified flag
        std::mem::replace(&mut self.title, title.into())
    }

    pub(crate) fn slot_slice(&self) -> &[Option<ItemStack>] {
        self.slots.as_ref()
    }
}

/// Send updates for each client's player inventory.
pub(crate) fn update_player_inventories(mut query: Query<(&mut Inventory, &mut Client)>) {
    for (mut inventory, mut client) in query.iter_mut() {
        if inventory.kind != InventoryKind::Player {
            warn!("Inventory on client entity is not a player inventory");
        }

        if inventory.modified != 0 {
            inventory.state_id += 1;

            if inventory.modified == u64::MAX {
                // Update the whole inventory.
                let cursor_item = client.cursor_item.clone();
                client.write_packet(&SetContainerContentEncode {
                    window_id: 0,
                    state_id: VarInt(inventory.state_id.0),
                    slots: inventory.slot_slice(),
                    carried_item: &cursor_item,
                });

                client.cursor_item_modified = false;
            } else {
                // Update only the slots that were modified.
                for (i, slot) in inventory.slots.iter().enumerate() {
                    if (inventory.modified >> i) & 1 == 1 {
                        client.write_packet(&SetContainerSlotEncode {
                            window_id: 0,
                            state_id: VarInt(inventory.state_id.0),
                            slot_idx: i as i16,
                            slot_data: slot.as_ref(),
                        });
                    }
                }
            }

            inventory.modified = 0;
        }

        if client.cursor_item_modified {
            inventory.state_id += 1;

            client.cursor_item_modified = false;

            let cursor_item = client.cursor_item.clone();
            client.write_packet(&SetContainerSlotEncode {
                window_id: -1,
                state_id: VarInt(inventory.state_id.0),
                slot_idx: -1,
                slot_data: cursor_item.as_ref(),
            });
        }
    }
}

/// Used to indicate that the client with this component is currently viewing
/// an inventory.
#[derive(Debug, Clone, Component)]
pub struct OpenInventory {
    /// The Entity with the `Inventory` component that the client is currently
    /// viewing.
    pub(crate) entity: Entity,
}

impl OpenInventory {
    pub fn new(entity: Entity) -> Self {
        OpenInventory { entity }
    }

    pub fn entity(&self) -> Entity {
        self.entity
    }
}

/// Handles the `OpenInventory` component being added to a client, which
/// indicates that the client is now viewing an inventory.
pub(crate) fn update_client_on_open_inventory(
    mut clients: Query<(&mut Client, &OpenInventory, Added<OpenInventory>)>,
    inventories: Query<&Inventory>,
) {
    for (mut client, open_inventory, _) in clients.iter_mut() {
        // validate that the inventory exists
        let inventory = inventories.get_component::<Inventory>(open_inventory.entity);
        if inventory.is_err() {
            warn!("Client is viewing an inventory that does not exist");
            continue;
        }

        // send the inventory to the client
        let inventory = inventory.unwrap();
        client.window_id = client.window_id % 100 + 1;

        let packet = OpenScreen {
            window_id: VarInt(client.window_id.into()),
            window_type: VarInt(WindowType::from(inventory.kind) as i32),
            window_title: inventory.title.clone(),
        };
        client.write_packet(&packet);

        let packet = SetContainerContentEncode {
            window_id: client.window_id,
            state_id: VarInt(inventory.state_id.0),
            slots: inventory.slot_slice(),
            carried_item: &client.cursor_item.clone(),
        };
        client.write_packet(&packet);
    }
}

pub(crate) fn update_open_inventories(
    mut commands: Commands,
    mut clients: Query<(Entity, &mut Client, &OpenInventory)>,
    mut inventories: Query<&mut Inventory>,
) {
    for (client_entity, mut client, open_inventory) in clients.iter_mut() {
        // validate that the inventory exists
        if let Ok(mut inventory) = inventories.get_component_mut::<Inventory>(open_inventory.entity)
        {
            // send the inventory to the client
            if inventory.modified == 0 {
                continue;
            }
            inventory.state_id += 1;

            let packet = SetContainerContentEncode {
                window_id: client.window_id,
                state_id: VarInt(inventory.state_id.0),
                slots: inventory.slot_slice(),
                carried_item: &client.cursor_item.clone(),
            };
            client.write_packet(&packet);

            inventory.modified = 0;
        } else {
            // the inventory no longer exists, so close the inventory
            commands.entity(client_entity).remove::<OpenInventory>();
        }
    }
}

/// Handles clients telling the server that they are closing an inventory.
pub(crate) fn handle_close_container(
    mut commands: Commands,
    mut events: EventReader<CloseContainer>,
) {
    for event in events.iter() {
        commands.entity(event.client).remove::<OpenInventory>();
    }
}

/// Detects when a client's `OpenInventory` component is removed, which
/// indicates that the client is no longer viewing an inventory.
pub(crate) fn update_client_on_close_inventory(
    removals: RemovedComponents<OpenInventory>,
    mut clients: Query<&mut Client>,
) {
    for entity in removals.iter() {
        if let Ok(mut client) = clients.get_component_mut::<Client>(entity) {
            let window_id = client.window_id;
            client.write_packet(&CloseContainerS2c { window_id });
        }
    }
}
