use std::iter::FusedIterator;
use std::num::Wrapping;

use bevy_ecs::prelude::*;
use valence_protocol::packets::s2c::play::{SetContainerContentEncode, SetContainerSlotEncode};
use valence_protocol::{InventoryKind, ItemStack, Text, VarInt};

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

pub(crate) fn update_player_inventories(mut query: Query<(&mut Inventory, &mut Client)>) {
    for (mut inventory, mut client) in query.iter_mut() {
        if inventory.modified != 0 {
            if inventory.modified == u64::MAX && client.cursor_item_modified {
                // Update the whole inventory.
                let cursor_item = client.cursor_item.clone();
                client.write_packet(&SetContainerContentEncode {
                    window_id: 0,
                    state_id: VarInt(inventory.state_id.0),
                    slots: inventory.slot_slice(),
                    carried_item: &cursor_item,
                });

                inventory.state_id += 1;
                client.cursor_item_modified = false;
            } else {
                // Update only the slots that were modified.
                let mut sent_updates = 0;
                for (i, slot) in inventory.slots.iter().enumerate() {
                    if (inventory.modified >> i) & 1 == 1 {
                        client.write_packet(&SetContainerSlotEncode {
                            window_id: 0,
                            state_id: VarInt(inventory.state_id.0 + sent_updates),
                            slot_idx: i as i16,
                            slot_data: slot.as_ref(),
                        });
                        sent_updates += 1;
                    }
                }
                inventory.state_id += sent_updates;
            }

            inventory.modified = 0;
        }

        if client.cursor_item_modified {
            client.cursor_item_modified = false;

            let cursor_item = client.cursor_item.clone();
            client.write_packet(&SetContainerSlotEncode {
                window_id: -1,
                state_id: VarInt(inventory.state_id.0),
                slot_idx: -1,
                slot_data: cursor_item.as_ref(),
            });

            inventory.state_id += 1;
        }
    }
}
