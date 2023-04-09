//! The inventory system.
//!
//! This module contains the systems and components needed to handle
//! inventories. By default, clients will have a player inventory attached to
//! them.
//!
//! # Components
//!
//! - [`Inventory`]: The inventory component. This is the thing that holds
//!   items.
//! - [`OpenInventory`]: The component that is attached to clients when they
//!   have an inventory open.
//!
//! # Examples
//!
//! An example system that will let you access all player's inventories:
//!
//! ```rust
//! # use valence::prelude::*;
//! fn system(mut clients: Query<(&Client, &Inventory)>) {}
//! ```
//!
//! ### See also
//!
//! Examples related to inventories in the `examples/` directory:
//! - `building`
//! - `chest`

use std::borrow::Cow;
use std::iter::FusedIterator;
use std::ops::Range;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use tracing::{debug, warn};
use valence_protocol::item::ItemStack;
use valence_protocol::packet::c2s::play::click_slot::{ClickMode, Slot};
use valence_protocol::packet::c2s::play::{
    ClickSlotC2s, CloseHandledScreenC2s, CreativeInventoryActionC2s, PlayerActionC2s,
    UpdateSelectedSlotC2s,
};
use valence_protocol::packet::s2c::play::{
    CloseScreenS2c, InventoryS2c, OpenScreenS2c, ScreenHandlerSlotUpdateS2c,
};
use valence_protocol::text::Text;
use valence_protocol::types::WindowType;
use valence_protocol::var_int::VarInt;

use crate::client::{Client, ClientInventoryState, CursorItem, FlushPacketsSet};
use crate::component::GameMode;
use crate::event_loop::{EventLoopSchedule, EventLoopSet, PacketEvent};
use crate::packet::WritePacket;

mod validate;

pub(crate) struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            (
                update_open_inventories,
                update_client_on_close_inventory.after(update_open_inventories),
                update_player_inventories,
            )
                .in_base_set(CoreSet::PostUpdate)
                .before(FlushPacketsSet),
        )
        .add_systems(
            (
                handle_update_selected_slot,
                handle_click_slot,
                handle_creative_inventory_action,
                handle_close_handled_screen,
                handle_player_actions,
            )
                .in_base_set(EventLoopSet::PreUpdate)
                .in_schedule(EventLoopSchedule),
        )
        .init_resource::<InventorySettings>()
        .add_event::<ClickSlot>()
        .add_event::<DropItemStack>()
        .add_event::<CreativeInventoryAction>()
        .add_event::<UpdateSelectedSlot>();
    }
}

/// The number of slots in the "main" part of the player inventory. 3 rows of 9,
/// plus the hotbar.
pub const PLAYER_INVENTORY_MAIN_SLOTS_COUNT: u16 = 36;

#[derive(Debug, Clone, Component)]
pub struct Inventory {
    title: Text,
    kind: InventoryKind,
    slots: Box<[Option<ItemStack>]>,
    /// Contains a set bit for each modified slot in `slots`.
    changed: u64,
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
            changed: 0,
        }
    }

    #[track_caller]
    pub fn slot(&self, idx: u16) -> Option<&ItemStack> {
        self.slots
            .get(idx as usize)
            .expect("slot index out of range")
            .as_ref()
    }

    /// Sets the slot at the given index to the given item stack.
    ///
    /// See also [`Inventory::replace_slot`].
    ///
    /// ```
    /// # use valence::prelude::*;
    /// let mut inv = Inventory::new(InventoryKind::Generic9x1);
    /// inv.set_slot(0, ItemStack::new(ItemKind::Diamond, 1, None));
    /// assert_eq!(inv.slot(0).unwrap().item, ItemKind::Diamond);
    /// ```
    #[track_caller]
    #[inline]
    pub fn set_slot(&mut self, idx: u16, item: impl Into<Option<ItemStack>>) {
        let _ = self.replace_slot(idx, item);
    }

    /// Replaces the slot at the given index with the given item stack, and
    /// returns the old stack in that slot.
    ///
    /// See also [`Inventory::set_slot`].
    ///
    /// ```
    /// # use valence::prelude::*;
    /// let mut inv = Inventory::new(InventoryKind::Generic9x1);
    /// inv.set_slot(0, ItemStack::new(ItemKind::Diamond, 1, None));
    /// let old = inv.replace_slot(0, ItemStack::new(ItemKind::IronIngot, 1, None));
    /// assert_eq!(old.unwrap().item, ItemKind::Diamond);
    /// ```
    #[track_caller]
    #[must_use]
    pub fn replace_slot(
        &mut self,
        idx: u16,
        item: impl Into<Option<ItemStack>>,
    ) -> Option<ItemStack> {
        assert!(idx < self.slot_count(), "slot index of {idx} out of bounds");

        let new = item.into();
        let old = &mut self.slots[idx as usize];

        if new != *old {
            self.changed |= 1 << idx;
        }

        std::mem::replace(old, new)
    }

    /// Swap the contents of two slots. If the slots are the same, nothing
    /// happens.
    ///
    /// ```
    /// # use valence::prelude::*;
    /// let mut inv = Inventory::new(InventoryKind::Generic9x1);
    /// inv.set_slot(0, ItemStack::new(ItemKind::Diamond, 1, None));
    /// assert_eq!(inv.slot(1), None);
    /// inv.swap_slot(0, 1);
    /// assert_eq!(inv.slot(1).unwrap().item, ItemKind::Diamond);
    /// ```
    #[track_caller]
    pub fn swap_slot(&mut self, idx_a: u16, idx_b: u16) {
        assert!(
            idx_a < self.slot_count(),
            "slot index of {idx_a} out of bounds"
        );
        assert!(
            idx_b < self.slot_count(),
            "slot index of {idx_b} out of bounds"
        );

        if idx_a == idx_b || self.slots[idx_a as usize] == self.slots[idx_b as usize] {
            // Nothing to do here, ignore.
            return;
        }

        self.changed |= 1 << idx_a;
        self.changed |= 1 << idx_b;

        self.slots.swap(idx_a as usize, idx_b as usize);
    }

    /// Set the amount of items in the given slot without replacing the slot
    /// entirely. Valid values are 1-127, inclusive, and `amount` will be
    /// clamped to this range. If the slot is empty, nothing happens.
    ///
    /// ```
    /// # use valence::prelude::*;
    /// let mut inv = Inventory::new(InventoryKind::Generic9x1);
    /// inv.set_slot(0, ItemStack::new(ItemKind::Diamond, 1, None));
    /// inv.set_slot_amount(0, 64);
    /// assert_eq!(inv.slot(0).unwrap().count(), 64);
    /// ```
    #[track_caller]
    pub fn set_slot_amount(&mut self, idx: u16, amount: u8) {
        assert!(idx < self.slot_count(), "slot index out of range");

        if let Some(item) = self.slots[idx as usize].as_mut() {
            if item.count() == amount {
                return;
            }
            item.set_count(amount);
            self.changed |= 1 << idx;
        }
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

    /// The text displayed on the inventory's title bar.
    ///
    /// ```
    /// # use valence::inventory::{Inventory, InventoryKind};
    /// # use valence_protocol::text::Text;
    /// let inv = Inventory::with_title(InventoryKind::Generic9x3, "Box of Holding");
    /// assert_eq!(inv.title(), &Text::from("Box of Holding"));
    /// ```
    pub fn title(&self) -> &Text {
        &self.title
    }

    /// Set the text displayed on the inventory's title bar.
    ///
    /// To get the old title, use [`Inventory::replace_title`].
    ///
    /// ```
    /// # use valence::inventory::{Inventory, InventoryKind};
    /// let mut inv = Inventory::new(InventoryKind::Generic9x3);
    /// inv.set_title("Box of Holding");
    /// ```
    #[inline]
    pub fn set_title(&mut self, title: impl Into<Text>) {
        let _ = self.replace_title(title);
    }

    /// Replace the text displayed on the inventory's title bar, and returns the
    /// old text.
    #[must_use]
    pub fn replace_title(&mut self, title: impl Into<Text>) -> Text {
        // TODO: set title modified flag
        std::mem::replace(&mut self.title, title.into())
    }

    pub(crate) fn slot_slice(&self) -> &[Option<ItemStack>] {
        self.slots.as_ref()
    }

    /// Returns the first empty slot in the given range, or `None` if there are
    /// no empty slots in the range.
    ///
    /// ```
    /// # use valence::prelude::*;
    /// let mut inv = Inventory::new(InventoryKind::Generic9x1);
    /// inv.set_slot(0, ItemStack::new(ItemKind::Diamond, 1, None));
    /// inv.set_slot(2, ItemStack::new(ItemKind::GoldIngot, 1, None));
    /// inv.set_slot(3, ItemStack::new(ItemKind::IronIngot, 1, None));
    /// assert_eq!(inv.first_empty_slot_in(0..6), Some(1));
    /// assert_eq!(inv.first_empty_slot_in(2..6), Some(4));
    /// ```
    #[track_caller]
    #[must_use]
    pub fn first_empty_slot_in(&self, mut range: Range<u16>) -> Option<u16> {
        assert!(
            (0..=self.slot_count()).contains(&range.start)
                && (0..=self.slot_count()).contains(&range.end),
            "slot range out of range"
        );

        range.find(|&idx| self.slots[idx as usize].is_none())
    }

    /// Returns the first empty slot in the inventory, or `None` if there are no
    /// empty slots.
    /// ```
    /// # use valence::prelude::*;
    /// let mut inv = Inventory::new(InventoryKind::Generic9x1);
    /// inv.set_slot(0, ItemStack::new(ItemKind::Diamond, 1, None));
    /// inv.set_slot(2, ItemStack::new(ItemKind::GoldIngot, 1, None));
    /// inv.set_slot(3, ItemStack::new(ItemKind::IronIngot, 1, None));
    /// assert_eq!(inv.first_empty_slot(), Some(1));
    /// ```
    pub fn first_empty_slot(&self) -> Option<u16> {
        self.first_empty_slot_in(0..self.slot_count())
    }
}

/// Used to indicate that the client with this component is currently viewing
/// an inventory.
#[derive(Component, Clone, Debug)]
pub struct OpenInventory {
    /// The entity with the `Inventory` component that the client is currently
    /// viewing.
    pub entity: Entity,
    client_changed: u64,
}

impl OpenInventory {
    pub fn new(entity: Entity) -> Self {
        OpenInventory {
            entity,
            client_changed: 0,
        }
    }
}

/// A helper to represent the inventory window that the player is currently
/// viewing. Handles dispatching reads to the correct inventory.
///
/// This is a read-only version of [`InventoryWindowMut`].
///
/// ```
/// # use valence::prelude::*;
/// let mut player_inventory = Inventory::new(InventoryKind::Player);
/// player_inventory.set_slot(36, ItemStack::new(ItemKind::Diamond, 1, None));
/// let target_inventory = Inventory::new(InventoryKind::Generic9x3);
/// let window = InventoryWindow::new(&player_inventory, Some(&target_inventory));
/// assert_eq!(
///     window.slot(54),
///     Some(&ItemStack::new(ItemKind::Diamond, 1, None))
/// );
/// ```
pub struct InventoryWindow<'a> {
    player_inventory: &'a Inventory,
    open_inventory: Option<&'a Inventory>,
}

impl<'a> InventoryWindow<'a> {
    pub fn new(player_inventory: &'a Inventory, open_inventory: Option<&'a Inventory>) -> Self {
        Self {
            player_inventory,
            open_inventory,
        }
    }

    #[track_caller]
    pub fn slot(&self, idx: u16) -> Option<&ItemStack> {
        if let Some(open_inv) = self.open_inventory.as_ref() {
            if idx < open_inv.slot_count() {
                return open_inv.slot(idx);
            } else {
                return self
                    .player_inventory
                    .slot(convert_to_player_slot_id(open_inv.kind(), idx));
            }
        } else {
            return self.player_inventory.slot(idx);
        }
    }

    #[track_caller]
    pub fn slot_count(&self) -> u16 {
        match self.open_inventory.as_ref() {
            Some(inv) => inv.slot_count() + PLAYER_INVENTORY_MAIN_SLOTS_COUNT,
            None => self.player_inventory.slot_count(),
        }
    }
}

/// A helper to represent the inventory window that the player is currently
/// viewing. Handles dispatching reads/writes to the correct inventory.
///
/// This is a writable version of [`InventoryWindow`].
///
/// ```
/// # use valence::prelude::*;
/// let mut player_inventory = Inventory::new(InventoryKind::Player);
/// let mut target_inventory = Inventory::new(InventoryKind::Generic9x3);
/// let mut window = InventoryWindowMut::new(&mut player_inventory, Some(&mut target_inventory));
/// window.set_slot(54, ItemStack::new(ItemKind::Diamond, 1, None));
/// assert_eq!(
///     player_inventory.slot(36),
///     Some(&ItemStack::new(ItemKind::Diamond, 1, None))
/// );
/// ```
pub struct InventoryWindowMut<'a> {
    player_inventory: &'a mut Inventory,
    open_inventory: Option<&'a mut Inventory>,
}

impl<'a> InventoryWindowMut<'a> {
    pub fn new(
        player_inventory: &'a mut Inventory,
        open_inventory: Option<&'a mut Inventory>,
    ) -> Self {
        Self {
            player_inventory,
            open_inventory,
        }
    }

    #[track_caller]
    pub fn slot(&self, idx: u16) -> Option<&ItemStack> {
        if let Some(open_inv) = self.open_inventory.as_ref() {
            if idx < open_inv.slot_count() {
                return open_inv.slot(idx);
            } else {
                return self
                    .player_inventory
                    .slot(convert_to_player_slot_id(open_inv.kind(), idx));
            }
        } else {
            return self.player_inventory.slot(idx);
        }
    }

    #[track_caller]
    #[must_use]
    pub fn replace_slot(
        &mut self,
        idx: u16,
        item: impl Into<Option<ItemStack>>,
    ) -> Option<ItemStack> {
        assert!(idx < self.slot_count(), "slot index of {idx} out of bounds");

        if let Some(open_inv) = self.open_inventory.as_mut() {
            if idx < open_inv.slot_count() {
                open_inv.replace_slot(idx, item)
            } else {
                self.player_inventory
                    .replace_slot(convert_to_player_slot_id(open_inv.kind(), idx), item)
            }
        } else {
            self.player_inventory.replace_slot(idx, item)
        }
    }

    #[track_caller]
    #[inline]
    pub fn set_slot(&mut self, idx: u16, item: impl Into<Option<ItemStack>>) {
        let _ = self.replace_slot(idx, item);
    }

    pub fn slot_count(&self) -> u16 {
        match self.open_inventory.as_ref() {
            Some(inv) => inv.slot_count() + PLAYER_INVENTORY_MAIN_SLOTS_COUNT,
            None => self.player_inventory.slot_count(),
        }
    }
}

/// Send updates for each client's player inventory.
fn update_player_inventories(
    mut query: Query<
        (
            &mut Inventory,
            &mut Client,
            &mut ClientInventoryState,
            Ref<CursorItem>,
        ),
        Without<OpenInventory>,
    >,
) {
    for (mut inventory, mut client, mut inv_state, cursor_item) in &mut query {
        if inventory.kind != InventoryKind::Player {
            warn!("Inventory on client entity is not a player inventory");
        }

        if inventory.changed == u64::MAX {
            // Update the whole inventory.

            inv_state.state_id += 1;

            client.write_packet(&InventoryS2c {
                window_id: 0,
                state_id: VarInt(inv_state.state_id.0),
                slots: Cow::Borrowed(inventory.slot_slice()),
                carried_item: Cow::Borrowed(&cursor_item.0),
            });

            inventory.changed = 0;
            inv_state.slots_changed = 0;

            // Skip updating the cursor item because we just updated the whole inventory.
            continue;
        } else if inventory.changed != 0 {
            // Send the modified slots.

            // The slots that were NOT modified by this client, and they need to be sent
            let changed_filtered = inventory.changed & !inv_state.slots_changed;

            if changed_filtered != 0 {
                inv_state.state_id += 1;

                for (i, slot) in inventory.slots.iter().enumerate() {
                    if ((changed_filtered >> i) & 1) == 1 {
                        client.write_packet(&ScreenHandlerSlotUpdateS2c {
                            window_id: 0,
                            state_id: VarInt(inv_state.state_id.0),
                            slot_idx: i as i16,
                            slot_data: Cow::Borrowed(slot),
                        });
                    }
                }
            }

            inventory.changed = 0;
            inv_state.slots_changed = 0;
        }

        if cursor_item.is_changed() && !inv_state.client_updated_cursor_item {
            // Contrary to what you might think, we actually don't want to increment the
            // state ID here because the client doesn't actually acknowledge the
            // state_id change for this packet specifically. See #304.

            client.write_packet(&ScreenHandlerSlotUpdateS2c {
                window_id: -1,
                state_id: VarInt(inv_state.state_id.0),
                slot_idx: -1,
                slot_data: Cow::Borrowed(&cursor_item.0),
            });
        }

        inv_state.client_updated_cursor_item = false;
    }
}

/// Handles the `OpenInventory` component being added to a client, which
/// indicates that the client is now viewing an inventory, and sends inventory
/// updates to the client when the inventory is modified.
fn update_open_inventories(
    mut clients: Query<(
        Entity,
        &mut Client,
        &mut ClientInventoryState,
        &CursorItem,
        &mut OpenInventory,
    )>,
    mut inventories: Query<&mut Inventory>,
    mut commands: Commands,
) {
    // These operations need to happen in this order.

    // Send the inventory contents to all clients that are viewing an inventory.
    for (client_entity, mut client, mut inv_state, cursor_item, mut open_inventory) in &mut clients
    {
        // Validate that the inventory exists.
        let Ok(mut inventory) = inventories.get_mut(open_inventory.entity) else {
            // The inventory no longer exists, so close the inventory.
            commands.entity(client_entity).remove::<OpenInventory>();

            client.write_packet(&CloseScreenS2c {
                window_id: inv_state.window_id,
            });

            continue;
        };

        if open_inventory.is_added() {
            // Send the inventory to the client if the client just opened the inventory.
            inv_state.window_id = inv_state.window_id % 100 + 1;
            open_inventory.client_changed = 0;

            client.write_packet(&OpenScreenS2c {
                window_id: VarInt(inv_state.window_id.into()),
                window_type: WindowType::from(inventory.kind),
                window_title: Cow::Borrowed(&inventory.title),
            });

            client.write_packet(&InventoryS2c {
                window_id: inv_state.window_id,
                state_id: VarInt(inv_state.state_id.0),
                slots: Cow::Borrowed(inventory.slot_slice()),
                carried_item: Cow::Borrowed(&cursor_item.0),
            });
        } else {
            // The client is already viewing the inventory.

            if inventory.changed == u64::MAX {
                // Send the entire inventory.

                inv_state.state_id += 1;

                client.write_packet(&InventoryS2c {
                    window_id: inv_state.window_id,
                    state_id: VarInt(inv_state.state_id.0),
                    slots: Cow::Borrowed(inventory.slot_slice()),
                    carried_item: Cow::Borrowed(&cursor_item.0),
                })
            } else {
                // Send the changed slots.

                // The slots that were NOT changed by this client, and they need to be sent.
                let changed_filtered = inventory.changed & !open_inventory.client_changed;

                if changed_filtered != 0 {
                    inv_state.state_id += 1;

                    for (i, slot) in inventory.slots.iter().enumerate() {
                        if (changed_filtered >> i) & 1 == 1 {
                            client.write_packet(&ScreenHandlerSlotUpdateS2c {
                                window_id: inv_state.window_id as i8,
                                state_id: VarInt(inv_state.state_id.0),
                                slot_idx: i as i16,
                                slot_data: Cow::Borrowed(slot),
                            });
                        }
                    }
                }
            }
        }

        open_inventory.client_changed = 0;
        inv_state.slots_changed = 0;
        inv_state.client_updated_cursor_item = false;
        inventory.changed = 0;
    }
}

/// Handles clients telling the server that they are closing an inventory.
fn handle_close_handled_screen(mut packets: EventReader<PacketEvent>, mut commands: Commands) {
    for packet in packets.iter() {
        if packet.decode::<CloseHandledScreenC2s>().is_some() {
            if let Some(mut entity) = commands.get_entity(packet.client) {
                entity.remove::<OpenInventory>();
            }
        }
    }
}

/// Detects when a client's `OpenInventory` component is removed, which
/// indicates that the client is no longer viewing an inventory.
fn update_client_on_close_inventory(
    mut removals: RemovedComponents<OpenInventory>,
    mut clients: Query<(&mut Client, &ClientInventoryState)>,
) {
    for entity in &mut removals {
        if let Ok((mut client, inv_state)) = clients.get_mut(entity) {
            client.write_packet(&CloseScreenS2c {
                window_id: inv_state.window_id,
            })
        }
    }
}

// TODO: make this event user friendly.
#[derive(Clone, Debug)]
pub struct ClickSlot {
    pub client: Entity,
    pub window_id: u8,
    pub state_id: i32,
    pub slot_id: i16,
    pub button: i8,
    pub mode: ClickMode,
    pub slot_changes: Vec<Slot>,
    pub carried_item: Option<ItemStack>,
}

#[derive(Clone, Debug)]
pub struct DropItemStack {
    pub client: Entity,
    pub from_slot: Option<u16>,
    pub stack: ItemStack,
}

fn handle_click_slot(
    mut packets: EventReader<PacketEvent>,
    mut clients: Query<(
        &mut Client,
        &mut Inventory,
        &mut ClientInventoryState,
        Option<&mut OpenInventory>,
        &mut CursorItem,
    )>,
    mut inventories: Query<&mut Inventory, Without<Client>>,
    mut drop_item_stack_events: EventWriter<DropItemStack>,
    mut click_slot_events: EventWriter<ClickSlot>,
) {
    for packet in packets.iter() {
        let Some(pkt) = packet.decode::<ClickSlotC2s>() else {
            // Not the packet we're looking for.
            continue
        };

        let Ok((
            mut client,
            mut client_inv,
            mut inv_state,
            open_inventory,
            mut cursor_item
        )) = clients.get_mut(packet.client) else {
            // The client does not exist, ignore.
            continue;
        };

        let open_inv = open_inventory
            .as_ref()
            .and_then(|open| inventories.get_mut(open.entity).ok());

        if let Err(e) = validate::validate_click_slot_packet(
            &pkt,
            &client_inv,
            open_inv.as_deref(),
            &cursor_item,
        ) {
            debug!(
                "failed to validate click slot packet for client {:#?}: \"{e:#}\" {pkt:#?}",
                packet.client
            );

            // Resync the inventory.

            client.write_packet(&InventoryS2c {
                window_id: if open_inv.is_some() {
                    inv_state.window_id
                } else {
                    0
                },
                state_id: VarInt(inv_state.state_id.0),
                slots: Cow::Borrowed(open_inv.unwrap_or(client_inv).slot_slice()),
                carried_item: Cow::Borrowed(&cursor_item.0),
            });

            continue;
        }

        if pkt.slot_idx < 0 && pkt.mode == ClickMode::Click {
            // The client is dropping the cursor item by clicking outside the window.

            if let Some(stack) = cursor_item.0.take() {
                drop_item_stack_events.send(DropItemStack {
                    client: packet.client,
                    from_slot: None,
                    stack,
                });
            }
        } else if pkt.mode == ClickMode::DropKey {
            // The client is dropping an item by pressing the drop key.

            let entire_stack = pkt.button == 1;
            if let Some(stack) = client_inv.slot(pkt.slot_idx as u16) {
                // TODO: is the use of `replace_slot` here causing unnecessary packets to be
                // sent?
                let dropped = if entire_stack || stack.count() == 1 {
                    client_inv.replace_slot(pkt.slot_idx as u16, None)
                } else {
                    let mut stack = stack.clone();
                    stack.set_count(stack.count() - 1);
                    let mut old_slot = client_inv.replace_slot(pkt.slot_idx as u16, Some(stack));
                    // we already checked that the slot was not empty and that the
                    // stack count is > 1
                    old_slot.as_mut().unwrap().set_count(1);
                    old_slot
                }
                .expect("dropped item should exist"); // we already checked that the slot was not empty

                drop_item_stack_events.send(DropItemStack {
                    client: packet.client,
                    from_slot: Some(pkt.slot_idx as u16),
                    stack: dropped,
                });
            }
        } else {
            // The player is clicking a slot in an inventory.

            // Validate the window id.
            if (pkt.window_id == 0) != open_inventory.is_none() {
                warn!(
                    "Client sent a click with an invalid window id for current state: window_id = \
                     {}, open_inventory present = {}",
                    pkt.window_id,
                    open_inventory.is_some()
                );
                continue;
            }

            if let Some(mut open_inventory) = open_inventory {
                // The player is interacting with an inventory that is open.

                let Ok(mut target_inventory) = inventories.get_mut(open_inventory.entity) else {
                    // The inventory does not exist, ignore.
                    continue;
                };

                if inv_state.state_id.0 != pkt.state_id.0 {
                    // Client is out of sync. Resync and ignore click.

                    debug!("Client state id mismatch, resyncing");

                    inv_state.state_id += 1;

                    client.write_packet(&InventoryS2c {
                        window_id: inv_state.window_id,
                        state_id: VarInt(inv_state.state_id.0),
                        slots: Cow::Borrowed(target_inventory.slot_slice()),
                        carried_item: Cow::Borrowed(&cursor_item.0),
                    });

                    continue;
                }

                cursor_item.set_if_neq(CursorItem(pkt.carried_item.clone()));

                for slot in pkt.slot_changes.clone() {
                    if (0i16..target_inventory.slot_count() as i16).contains(&slot.idx) {
                        // The client is interacting with a slot in the target inventory.
                        target_inventory.set_slot(slot.idx as u16, slot.item);
                        open_inventory.client_changed |= 1 << slot.idx;
                    } else {
                        // The client is interacting with a slot in their own inventory.
                        let slot_id =
                            convert_to_player_slot_id(target_inventory.kind, slot.idx as u16);
                        client_inv.set_slot(slot_id, slot.item);
                        inv_state.slots_changed |= 1 << slot_id;
                    }
                }
            } else {
                // The client is interacting with their own inventory.

                if inv_state.state_id.0 != pkt.state_id.0 {
                    // Client is out of sync. Resync and ignore the click.

                    debug!("Client state id mismatch, resyncing");

                    inv_state.state_id += 1;

                    client.write_packet(&InventoryS2c {
                        window_id: inv_state.window_id,
                        state_id: VarInt(inv_state.state_id.0),
                        slots: Cow::Borrowed(client_inv.slot_slice()),
                        carried_item: Cow::Borrowed(&cursor_item.0),
                    });

                    continue;
                }

                cursor_item.set_if_neq(CursorItem(pkt.carried_item.clone()));
                inv_state.client_updated_cursor_item = true;

                for slot in pkt.slot_changes.clone() {
                    if (0i16..client_inv.slot_count() as i16).contains(&slot.idx) {
                        client_inv.set_slot(slot.idx as u16, slot.item);
                        inv_state.slots_changed |= 1 << slot.idx;
                    } else {
                        // The client is trying to interact with a slot that does not exist,
                        // ignore.
                        warn!(
                            "Client attempted to interact with slot {} which does not exist",
                            slot.idx
                        );
                    }
                }
            }

            click_slot_events.send(ClickSlot {
                client: packet.client,
                window_id: pkt.window_id,
                state_id: pkt.state_id.0,
                slot_id: pkt.slot_idx,
                button: pkt.button,
                mode: pkt.mode,
                slot_changes: pkt.slot_changes,
                carried_item: pkt.carried_item,
            });
        }
    }
}

fn handle_player_actions(
    mut packets: EventReader<PacketEvent>,
    mut clients: Query<(&mut Inventory, &mut ClientInventoryState)>,
    mut drop_item_stack_events: EventWriter<DropItemStack>,
) {
    for packet in packets.iter() {
        if let Some(pkt) = packet.decode::<PlayerActionC2s>() {
            use valence_protocol::packet::c2s::play::player_action::Action;

            match pkt.action {
                Action::DropAllItems => {
                    if let Ok((mut inv, mut inv_state)) = clients.get_mut(packet.client) {
                        if let Some(stack) = inv.replace_slot(inv_state.held_item_slot, None) {
                            inv_state.slots_changed |= 1 << inv_state.held_item_slot;

                            drop_item_stack_events.send(DropItemStack {
                                client: packet.client,
                                from_slot: Some(inv_state.held_item_slot),
                                stack,
                            });
                        }
                    }
                }
                Action::DropItem => {
                    if let Ok((mut inv, mut inv_state)) = clients.get_mut(packet.client) {
                        if let Some(mut stack) = inv.replace_slot(inv_state.held_item_slot(), None)
                        {
                            if stack.count() > 1 {
                                inv.set_slot(
                                    inv_state.held_item_slot(),
                                    stack.clone().with_count(stack.count() - 1),
                                );

                                stack.set_count(1);
                            }

                            inv_state.slots_changed |= 1 << inv_state.held_item_slot();

                            drop_item_stack_events.send(DropItemStack {
                                client: packet.client,
                                from_slot: Some(inv_state.held_item_slot()),
                                stack,
                            })
                        }
                    }
                }
                Action::SwapItemWithOffhand => {
                    // TODO
                }
                _ => {}
            }
        }
    }
}

// TODO: make this event user friendly.
#[derive(Clone, Debug)]
pub struct CreativeInventoryAction {
    pub client: Entity,
    pub slot: i16,
    pub clicked_item: Option<ItemStack>,
}

fn handle_creative_inventory_action(
    mut packets: EventReader<PacketEvent>,
    mut clients: Query<(
        &mut Client,
        &mut Inventory,
        &mut ClientInventoryState,
        &GameMode,
    )>,
    mut inv_action_events: EventWriter<CreativeInventoryAction>,
    mut drop_item_stack_events: EventWriter<DropItemStack>,
) {
    for packet in packets.iter() {
        if let Some(pkt) = packet.decode::<CreativeInventoryActionC2s>() {
            let Ok((mut client, mut inventory, mut inv_state, game_mode)) = clients.get_mut(packet.client) else {
                continue
            };

            if *game_mode != GameMode::Creative {
                // The client is not in creative mode, ignore.
                continue;
            }

            if pkt.slot == -1 {
                if let Some(stack) = pkt.clicked_item.clone() {
                    drop_item_stack_events.send(DropItemStack {
                        client: packet.client,
                        from_slot: None,
                        stack,
                    });
                }
                continue;
            }

            if pkt.slot < 0 || pkt.slot >= inventory.slot_count() as i16 {
                // The client is trying to interact with a slot that does not exist, ignore.
                continue;
            }

            // Set the slot without marking it as changed.
            inventory.slots[pkt.slot as usize] = pkt.clicked_item.clone();

            inv_state.state_id += 1;

            // HACK: notchian clients rely on the server to send the slot update when in
            // creative mode. Simply marking the slot as changed is not enough. This was
            // discovered because shift-clicking the destroy item slot in creative mode does
            // not work without this hack.
            client.write_packet(&ScreenHandlerSlotUpdateS2c {
                window_id: 0,
                state_id: VarInt(inv_state.state_id.0),
                slot_idx: pkt.slot,
                slot_data: Cow::Borrowed(&pkt.clicked_item),
            });

            inv_action_events.send(CreativeInventoryAction {
                client: packet.client,
                slot: pkt.slot,
                clicked_item: pkt.clicked_item,
            });
        }
    }
}

#[derive(Clone, Debug)]
pub struct UpdateSelectedSlot {
    pub client: Entity,
    pub slot: i16,
}

fn handle_update_selected_slot(
    mut packets: EventReader<PacketEvent>,
    mut clients: Query<&mut ClientInventoryState>,
    mut events: EventWriter<UpdateSelectedSlot>,
) {
    for packet in packets.iter() {
        if let Some(pkt) = packet.decode::<UpdateSelectedSlotC2s>() {
            if let Ok(mut inv_state) = clients.get_mut(packet.client) {
                // TODO: validate this.
                inv_state.held_item_slot = convert_hotbar_slot_id(pkt.slot as u16);

                events.send(UpdateSelectedSlot {
                    client: packet.client,
                    slot: pkt.slot,
                });
            }
        }
    }
}

/// Convert a slot that is outside a target inventory's range to a slot that is
/// inside the player's inventory.
fn convert_to_player_slot_id(target_kind: InventoryKind, slot_id: u16) -> u16 {
    // the first slot in the player's general inventory
    let offset = target_kind.slot_count() as u16;
    slot_id - offset + 9
}

fn convert_hotbar_slot_id(slot_id: u16) -> u16 {
    slot_id + PLAYER_INVENTORY_MAIN_SLOTS_COUNT
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum InventoryKind {
    Generic9x1,
    Generic9x2,
    Generic9x3,
    Generic9x4,
    Generic9x5,
    Generic9x6,
    Generic3x3,
    Anvil,
    Beacon,
    BlastFurnace,
    BrewingStand,
    Crafting,
    Enchantment,
    Furnace,
    Grindstone,
    Hopper,
    Lectern,
    Loom,
    Merchant,
    ShulkerBox,
    Smithing,
    Smoker,
    Cartography,
    Stonecutter,
    Player,
}

impl InventoryKind {
    /// The number of slots in this inventory. When the inventory is shown to
    /// clients, this number does not include the player's main inventory slots.
    pub const fn slot_count(self) -> usize {
        match self {
            InventoryKind::Generic9x1 => 9,
            InventoryKind::Generic9x2 => 9 * 2,
            InventoryKind::Generic9x3 => 9 * 3,
            InventoryKind::Generic9x4 => 9 * 4,
            InventoryKind::Generic9x5 => 9 * 5,
            InventoryKind::Generic9x6 => 9 * 6,
            InventoryKind::Generic3x3 => 3 * 3,
            InventoryKind::Anvil => 4,
            InventoryKind::Beacon => 1,
            InventoryKind::BlastFurnace => 3,
            InventoryKind::BrewingStand => 5,
            InventoryKind::Crafting => 10,
            InventoryKind::Enchantment => 2,
            InventoryKind::Furnace => 3,
            InventoryKind::Grindstone => 3,
            InventoryKind::Hopper => 5,
            InventoryKind::Lectern => 1,
            InventoryKind::Loom => 4,
            InventoryKind::Merchant => 3,
            InventoryKind::ShulkerBox => 27,
            InventoryKind::Smithing => 3,
            InventoryKind::Smoker => 3,
            InventoryKind::Cartography => 3,
            InventoryKind::Stonecutter => 2,
            InventoryKind::Player => 46,
        }
    }
}

impl From<InventoryKind> for WindowType {
    fn from(value: InventoryKind) -> Self {
        match value {
            InventoryKind::Generic9x1 => WindowType::Generic9x1,
            InventoryKind::Generic9x2 => WindowType::Generic9x2,
            InventoryKind::Generic9x3 => WindowType::Generic9x3,
            InventoryKind::Generic9x4 => WindowType::Generic9x4,
            InventoryKind::Generic9x5 => WindowType::Generic9x5,
            InventoryKind::Generic9x6 => WindowType::Generic9x6,
            InventoryKind::Generic3x3 => WindowType::Generic3x3,
            InventoryKind::Anvil => WindowType::Anvil,
            InventoryKind::Beacon => WindowType::Beacon,
            InventoryKind::BlastFurnace => WindowType::BlastFurnace,
            InventoryKind::BrewingStand => WindowType::BrewingStand,
            InventoryKind::Crafting => WindowType::Crafting,
            InventoryKind::Enchantment => WindowType::Enchantment,
            InventoryKind::Furnace => WindowType::Furnace,
            InventoryKind::Grindstone => WindowType::Grindstone,
            InventoryKind::Hopper => WindowType::Hopper,
            InventoryKind::Lectern => WindowType::Lectern,
            InventoryKind::Loom => WindowType::Loom,
            InventoryKind::Merchant => WindowType::Merchant,
            InventoryKind::ShulkerBox => WindowType::ShulkerBox,
            InventoryKind::Smithing => WindowType::Smithing,
            InventoryKind::Smoker => WindowType::Smoker,
            InventoryKind::Cartography => WindowType::Cartography,
            InventoryKind::Stonecutter => WindowType::Stonecutter,
            // arbitrarily chosen, because a player inventory technically does not have a window
            // type
            InventoryKind::Player => WindowType::Generic9x4,
        }
    }
}

impl From<WindowType> for InventoryKind {
    fn from(value: WindowType) -> Self {
        match value {
            WindowType::Generic9x1 => InventoryKind::Generic9x1,
            WindowType::Generic9x2 => InventoryKind::Generic9x2,
            WindowType::Generic9x3 => InventoryKind::Generic9x3,
            WindowType::Generic9x4 => InventoryKind::Generic9x4,
            WindowType::Generic9x5 => InventoryKind::Generic9x5,
            WindowType::Generic9x6 => InventoryKind::Generic9x6,
            WindowType::Generic3x3 => InventoryKind::Generic3x3,
            WindowType::Anvil => InventoryKind::Anvil,
            WindowType::Beacon => InventoryKind::Beacon,
            WindowType::BlastFurnace => InventoryKind::BlastFurnace,
            WindowType::BrewingStand => InventoryKind::BrewingStand,
            WindowType::Crafting => InventoryKind::Crafting,
            WindowType::Enchantment => InventoryKind::Enchantment,
            WindowType::Furnace => InventoryKind::Furnace,
            WindowType::Grindstone => InventoryKind::Grindstone,
            WindowType::Hopper => InventoryKind::Hopper,
            WindowType::Lectern => InventoryKind::Lectern,
            WindowType::Loom => InventoryKind::Loom,
            WindowType::Merchant => InventoryKind::Merchant,
            WindowType::ShulkerBox => InventoryKind::ShulkerBox,
            WindowType::Smithing => InventoryKind::Smithing,
            WindowType::Smoker => InventoryKind::Smoker,
            WindowType::Cartography => InventoryKind::Cartography,
            WindowType::Stonecutter => InventoryKind::Stonecutter,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Resource)]
pub struct InventorySettings {
    pub validate_actions: bool,
}

impl Default for InventorySettings {
    fn default() -> Self {
        Self {
            validate_actions: true,
        }
    }
}

#[cfg(test)]
mod test {
    use bevy_app::App;
    use valence_protocol::item::ItemKind;
    use valence_protocol::packet::c2s::play::click_slot::{ClickMode, Slot};
    use valence_protocol::packet::c2s::play::ClickSlotC2s;
    use valence_protocol::packet::S2cPlayPacket;

    use super::*;
    use crate::unit_test::util::scenario_single_client;
    use crate::{assert_packet_count, assert_packet_order};

    #[test]
    fn test_convert_to_player_slot() {
        assert_eq!(convert_to_player_slot_id(InventoryKind::Generic9x3, 27), 9);
        assert_eq!(convert_to_player_slot_id(InventoryKind::Generic9x3, 36), 18);
        assert_eq!(convert_to_player_slot_id(InventoryKind::Generic9x3, 54), 36);
        assert_eq!(convert_to_player_slot_id(InventoryKind::Generic9x1, 9), 9);
    }

    #[test]
    fn test_convert_hotbar_slot_id() {
        assert_eq!(convert_hotbar_slot_id(0), 36);
        assert_eq!(convert_hotbar_slot_id(4), 40);
        assert_eq!(convert_hotbar_slot_id(8), 44);
    }

    #[test]
    fn test_should_open_inventory() {
        let mut app = App::new();
        let (client_ent, mut client_helper) = scenario_single_client(&mut app);

        let inventory = Inventory::new(InventoryKind::Generic3x3);
        let inventory_ent = app.world.spawn(inventory).id();

        // Process a tick to get past the "on join" logic.
        app.update();
        client_helper.clear_sent();

        // Open the inventory.
        let open_inventory = OpenInventory::new(inventory_ent);
        app.world
            .get_entity_mut(client_ent)
            .expect("could not find client")
            .insert(open_inventory);

        app.update();

        // Make assertions
        let sent_packets = client_helper.collect_sent();

        assert_packet_count!(sent_packets, 1, S2cPlayPacket::OpenScreenS2c(_));
        assert_packet_count!(sent_packets, 1, S2cPlayPacket::InventoryS2c(_));
        assert_packet_order!(
            sent_packets,
            S2cPlayPacket::OpenScreenS2c(_),
            S2cPlayPacket::InventoryS2c(_)
        );
    }

    #[test]
    fn test_should_close_inventory() {
        let mut app = App::new();
        let (client_ent, mut client_helper) = scenario_single_client(&mut app);

        let inventory = Inventory::new(InventoryKind::Generic3x3);
        let inventory_ent = app.world.spawn(inventory).id();

        // Process a tick to get past the "on join" logic.
        app.update();
        client_helper.clear_sent();

        // Open the inventory.
        let open_inventory = OpenInventory::new(inventory_ent);
        app.world
            .get_entity_mut(client_ent)
            .expect("could not find client")
            .insert(open_inventory);

        app.update();
        client_helper.clear_sent();

        // Close the inventory.
        app.world
            .get_entity_mut(client_ent)
            .expect("could not find client")
            .remove::<OpenInventory>();

        app.update();

        // Make assertions
        let sent_packets = client_helper.collect_sent();

        assert_packet_count!(sent_packets, 1, S2cPlayPacket::CloseScreenS2c(_));
    }

    #[test]
    fn test_should_remove_invalid_open_inventory() {
        let mut app = App::new();
        let (client_ent, mut client_helper) = scenario_single_client(&mut app);

        let inventory = Inventory::new(InventoryKind::Generic3x3);
        let inventory_ent = app.world.spawn(inventory).id();

        // Process a tick to get past the "on join" logic.
        app.update();
        client_helper.clear_sent();

        // Open the inventory.
        let open_inventory = OpenInventory::new(inventory_ent);
        app.world
            .get_entity_mut(client_ent)
            .expect("could not find client")
            .insert(open_inventory);

        app.update();
        client_helper.clear_sent();

        // Remove the inventory.
        app.world.despawn(inventory_ent);

        app.update();

        // Make assertions
        assert!(app.world.get::<OpenInventory>(client_ent).is_none());
        let sent_packets = client_helper.collect_sent();
        assert_packet_count!(sent_packets, 1, S2cPlayPacket::CloseScreenS2c(_));
    }

    #[test]
    fn test_should_modify_player_inventory_click_slot() {
        let mut app = App::new();
        let (client_ent, mut client_helper) = scenario_single_client(&mut app);
        let mut inventory = app
            .world
            .get_mut::<Inventory>(client_ent)
            .expect("could not find inventory for client");
        inventory.set_slot(20, ItemStack::new(ItemKind::Diamond, 2, None));

        // Process a tick to get past the "on join" logic.
        app.update();
        client_helper.clear_sent();

        // Make the client click the slot and pick up the item.
        let state_id = app
            .world
            .get::<ClientInventoryState>(client_ent)
            .unwrap()
            .state_id;
        client_helper.send(&valence_protocol::packet::c2s::play::ClickSlotC2s {
            window_id: 0,
            button: 0,
            mode: valence_protocol::packet::c2s::play::click_slot::ClickMode::Click,
            state_id: VarInt(state_id.0),
            slot_idx: 20,
            slot_changes: vec![valence_protocol::packet::c2s::play::click_slot::Slot {
                idx: 20,
                item: None,
            }],
            carried_item: Some(ItemStack::new(ItemKind::Diamond, 2, None)),
        });

        app.update();

        // Make assertions
        let sent_packets = client_helper.collect_sent();

        // because the inventory was changed as a result of the client's click, the
        // server should not send any packets to the client because the client
        // already knows about the change.
        assert_packet_count!(
            sent_packets,
            0,
            S2cPlayPacket::InventoryS2c(_) | S2cPlayPacket::ScreenHandlerSlotUpdateS2c(_)
        );
        let inventory = app
            .world
            .get::<Inventory>(client_ent)
            .expect("could not find inventory for client");
        assert_eq!(inventory.slot(20), None);
        let cursor_item = app
            .world
            .get::<CursorItem>(client_ent)
            .expect("could not find client");
        assert_eq!(
            cursor_item.0,
            Some(ItemStack::new(ItemKind::Diamond, 2, None))
        );
    }

    #[test]
    fn test_should_modify_player_inventory_server_side() {
        let mut app = App::new();
        let (client_ent, mut client_helper) = scenario_single_client(&mut app);
        let mut inventory = app
            .world
            .get_mut::<Inventory>(client_ent)
            .expect("could not find inventory for client");
        inventory.set_slot(20, ItemStack::new(ItemKind::Diamond, 2, None));

        // Process a tick to get past the "on join" logic.
        app.update();
        client_helper.clear_sent();

        // Modify the inventory.
        let mut inventory = app
            .world
            .get_mut::<Inventory>(client_ent)
            .expect("could not find inventory for client");
        inventory.set_slot(21, ItemStack::new(ItemKind::IronIngot, 1, None));

        app.update();

        // Make assertions
        let sent_packets = client_helper.collect_sent();
        // because the inventory was modified server side, the client needs to be
        // updated with the change.
        assert_packet_count!(
            sent_packets,
            1,
            S2cPlayPacket::ScreenHandlerSlotUpdateS2c(_)
        );
    }

    #[test]
    fn test_should_sync_entire_player_inventory() {
        let mut app = App::new();
        let (client_ent, mut client_helper) = scenario_single_client(&mut app);

        // Process a tick to get past the "on join" logic.
        app.update();
        client_helper.clear_sent();

        let mut inventory = app
            .world
            .get_mut::<Inventory>(client_ent)
            .expect("could not find inventory for client");
        inventory.changed = u64::MAX;

        app.update();

        // Make assertions
        let sent_packets = client_helper.collect_sent();
        assert_packet_count!(sent_packets, 1, S2cPlayPacket::InventoryS2c(_));
    }

    fn set_up_open_inventory(app: &mut App, client_ent: Entity) -> Entity {
        let inventory = Inventory::new(InventoryKind::Generic9x3);
        let inventory_ent = app.world.spawn(inventory).id();

        // Open the inventory.
        let open_inventory = OpenInventory::new(inventory_ent);
        app.world
            .get_entity_mut(client_ent)
            .expect("could not find client")
            .insert(open_inventory);

        inventory_ent
    }

    #[test]
    fn test_should_modify_open_inventory_click_slot() {
        let mut app = App::new();
        let (client_ent, mut client_helper) = scenario_single_client(&mut app);
        let inventory_ent = set_up_open_inventory(&mut app, client_ent);
        let mut inventory = app
            .world
            .get_mut::<Inventory>(inventory_ent)
            .expect("could not find inventory for client");
        inventory.set_slot(20, ItemStack::new(ItemKind::Diamond, 2, None));

        // Process a tick to get past the "on join" logic.
        app.update();
        client_helper.clear_sent();

        // Make the client click the slot and pick up the item.
        let inv_state = app.world.get::<ClientInventoryState>(client_ent).unwrap();
        let state_id = inv_state.state_id;
        let window_id = inv_state.window_id;
        client_helper.send(&valence_protocol::packet::c2s::play::ClickSlotC2s {
            window_id,
            button: 0,
            mode: valence_protocol::packet::c2s::play::click_slot::ClickMode::Click,
            state_id: VarInt(state_id.0),
            slot_idx: 20,
            slot_changes: vec![valence_protocol::packet::c2s::play::click_slot::Slot {
                idx: 20,
                item: None,
            }],
            carried_item: Some(ItemStack::new(ItemKind::Diamond, 2, None)),
        });

        app.update();

        // Make assertions
        let sent_packets = client_helper.collect_sent();

        // because the inventory was modified as a result of the client's click, the
        // server should not send any packets to the client because the client
        // already knows about the change.
        assert_packet_count!(
            sent_packets,
            0,
            S2cPlayPacket::InventoryS2c(_) | S2cPlayPacket::ScreenHandlerSlotUpdateS2c(_)
        );
        let inventory = app
            .world
            .get::<Inventory>(inventory_ent)
            .expect("could not find inventory");
        assert_eq!(inventory.slot(20), None);
        let cursor_item = app
            .world
            .get::<CursorItem>(client_ent)
            .expect("could not find client");
        assert_eq!(
            cursor_item.0,
            Some(ItemStack::new(ItemKind::Diamond, 2, None))
        );
    }

    #[test]
    fn test_should_modify_open_inventory_server_side() {
        let mut app = App::new();
        let (client_ent, mut client_helper) = scenario_single_client(&mut app);
        let inventory_ent = set_up_open_inventory(&mut app, client_ent);

        // Process a tick to get past the "on join" logic.
        app.update();
        client_helper.clear_sent();

        // Modify the inventory.
        let mut inventory = app
            .world
            .get_mut::<Inventory>(inventory_ent)
            .expect("could not find inventory for client");
        inventory.set_slot(5, ItemStack::new(ItemKind::IronIngot, 1, None));

        app.update();

        // Make assertions
        let sent_packets = client_helper.collect_sent();

        // because the inventory was modified server side, the client needs to be
        // updated with the change.
        assert_packet_count!(
            sent_packets,
            1,
            S2cPlayPacket::ScreenHandlerSlotUpdateS2c(_)
        );
        let inventory = app
            .world
            .get::<Inventory>(inventory_ent)
            .expect("could not find inventory for client");
        assert_eq!(
            inventory.slot(5),
            Some(&ItemStack::new(ItemKind::IronIngot, 1, None))
        );
    }

    #[test]
    fn test_should_sync_entire_open_inventory() {
        let mut app = App::new();
        let (client_ent, mut client_helper) = scenario_single_client(&mut app);
        let inventory_ent = set_up_open_inventory(&mut app, client_ent);

        // Process a tick to get past the "on join" logic.
        app.update();
        client_helper.clear_sent();

        let mut inventory = app
            .world
            .get_mut::<Inventory>(inventory_ent)
            .expect("could not find inventory");
        inventory.changed = u64::MAX;

        app.update();

        // Make assertions
        let sent_packets = client_helper.collect_sent();
        assert_packet_count!(sent_packets, 1, S2cPlayPacket::InventoryS2c(_));
    }

    #[test]
    fn test_set_creative_mode_slot_handling() {
        let mut app = App::new();
        let (client_ent, mut client_helper) = scenario_single_client(&mut app);
        let mut game_mode = app
            .world
            .get_mut::<GameMode>(client_ent)
            .expect("could not find client");
        *game_mode.as_mut() = GameMode::Creative;

        // Process a tick to get past the "on join" logic.
        app.update();
        client_helper.clear_sent();

        client_helper.send(
            &valence_protocol::packet::c2s::play::CreativeInventoryActionC2s {
                slot: 36,
                clicked_item: Some(ItemStack::new(ItemKind::Diamond, 2, None)),
            },
        );

        app.update();

        // Make assertions
        let inventory = app
            .world
            .get::<Inventory>(client_ent)
            .expect("could not find inventory for client");
        assert_eq!(
            inventory.slot(36),
            Some(&ItemStack::new(ItemKind::Diamond, 2, None))
        );
    }

    #[test]
    fn test_ignore_set_creative_mode_slot_if_not_creative() {
        let mut app = App::new();
        let (client_ent, mut client_helper) = scenario_single_client(&mut app);
        let mut game_mode = app
            .world
            .get_mut::<GameMode>(client_ent)
            .expect("could not find client");
        *game_mode.as_mut() = GameMode::Survival;

        // Process a tick to get past the "on join" logic.
        app.update();
        client_helper.clear_sent();

        client_helper.send(
            &valence_protocol::packet::c2s::play::CreativeInventoryActionC2s {
                slot: 36,
                clicked_item: Some(ItemStack::new(ItemKind::Diamond, 2, None)),
            },
        );

        app.update();

        // Make assertions
        let inventory = app
            .world
            .get::<Inventory>(client_ent)
            .expect("could not find inventory for client");
        assert_eq!(inventory.slot(36), None);
    }

    #[test]
    fn test_window_id_increments() {
        let mut app = App::new();
        let (client_ent, mut client_helper) = scenario_single_client(&mut app);
        let inventory = Inventory::new(InventoryKind::Generic9x3);
        let inventory_ent = app.world.spawn(inventory).id();

        // Process a tick to get past the "on join" logic.
        app.update();
        client_helper.clear_sent();

        for _ in 0..3 {
            let open_inventory = OpenInventory::new(inventory_ent);
            app.world
                .get_entity_mut(client_ent)
                .expect("could not find client")
                .insert(open_inventory);

            app.update();

            app.world
                .get_entity_mut(client_ent)
                .expect("could not find client")
                .remove::<OpenInventory>();

            app.update();
        }

        // Make assertions
        let inv_state = app
            .world
            .get::<ClientInventoryState>(client_ent)
            .expect("could not find client");
        assert_eq!(inv_state.window_id, 3);
    }

    #[test]
    fn test_should_handle_set_held_item() {
        let mut app = App::new();
        let (client_ent, mut client_helper) = scenario_single_client(&mut app);

        // Process a tick to get past the "on join" logic.
        app.update();
        client_helper.clear_sent();

        client_helper.send(&valence_protocol::packet::c2s::play::UpdateSelectedSlotC2s { slot: 4 });

        app.update();

        // Make assertions
        let inv_state = app
            .world
            .get::<ClientInventoryState>(client_ent)
            .expect("could not find client");
        assert_eq!(inv_state.held_item_slot, 40);
    }

    #[test]
    fn should_not_increment_state_id_on_cursor_item_change() {
        let mut app = App::new();
        let (client_ent, mut client_helper) = scenario_single_client(&mut app);

        let inv_state = app
            .world
            .get::<ClientInventoryState>(client_ent)
            .expect("could not find client");
        let expected_state_id = inv_state.state_id.0;

        // Process a tick to get past the "on join" logic.
        app.update();
        client_helper.clear_sent();

        let mut cursor_item = app.world.get_mut::<CursorItem>(client_ent).unwrap();
        cursor_item.0 = Some(ItemStack::new(ItemKind::Diamond, 2, None));

        app.update();

        // Make assertions
        let inv_state = app
            .world
            .get::<ClientInventoryState>(client_ent)
            .expect("could not find client");
        assert_eq!(
            inv_state.state_id.0, expected_state_id,
            "state id should not have changed"
        );
    }

    mod dropping_items {
        use valence_protocol::block_pos::BlockPos;
        use valence_protocol::packet::c2s::play::click_slot::{ClickMode, Slot};
        use valence_protocol::packet::c2s::play::player_action::Action;
        use valence_protocol::types::Direction;

        use super::*;

        #[test]
        fn should_drop_item_player_action() {
            let mut app = App::new();
            let (client_ent, mut client_helper) = scenario_single_client(&mut app);
            let mut inventory = app
                .world
                .get_mut::<Inventory>(client_ent)
                .expect("could not find inventory");
            inventory.set_slot(36, ItemStack::new(ItemKind::IronIngot, 3, None));

            // Process a tick to get past the "on join" logic.
            app.update();
            client_helper.clear_sent();

            client_helper.send(&valence_protocol::packet::c2s::play::PlayerActionC2s {
                action: Action::DropItem,
                position: BlockPos::new(0, 0, 0),
                direction: Direction::Down,
                sequence: VarInt(0),
            });

            app.update();

            // Make assertions
            let inventory = app
                .world
                .get::<Inventory>(client_ent)
                .expect("could not find client");
            assert_eq!(
                inventory.slot(36),
                Some(&ItemStack::new(ItemKind::IronIngot, 2, None))
            );
            let events = app
                .world
                .get_resource::<Events<DropItemStack>>()
                .expect("expected drop item stack events");
            let events = events.iter_current_update_events().collect::<Vec<_>>();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].client, client_ent);
            assert_eq!(events[0].from_slot, Some(36));
            assert_eq!(
                events[0].stack,
                ItemStack::new(ItemKind::IronIngot, 1, None)
            );

            let sent_packets = client_helper.collect_sent();
            assert_packet_count!(
                sent_packets,
                0,
                S2cPlayPacket::ScreenHandlerSlotUpdateS2c(_)
            );
        }

        #[test]
        fn should_drop_item_stack_player_action() {
            let mut app = App::new();
            let (client_ent, mut client_helper) = scenario_single_client(&mut app);
            let mut inventory = app
                .world
                .get_mut::<Inventory>(client_ent)
                .expect("could not find inventory");
            inventory.set_slot(36, ItemStack::new(ItemKind::IronIngot, 32, None));

            // Process a tick to get past the "on join" logic.
            app.update();
            client_helper.clear_sent();

            client_helper.send(&valence_protocol::packet::c2s::play::PlayerActionC2s {
                action: Action::DropAllItems,
                position: BlockPos::new(0, 0, 0),
                direction: Direction::Down,
                sequence: VarInt(0),
            });

            app.update();

            // Make assertions
            let inv_state = app
                .world
                .get::<ClientInventoryState>(client_ent)
                .expect("could not find client");
            assert_eq!(inv_state.held_item_slot, 36);
            let inventory = app
                .world
                .get::<Inventory>(client_ent)
                .expect("could not find inventory");
            assert_eq!(inventory.slot(36), None);
            let events = app
                .world
                .get_resource::<Events<DropItemStack>>()
                .expect("expected drop item stack events");
            let events = events.iter_current_update_events().collect::<Vec<_>>();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].client, client_ent);
            assert_eq!(events[0].from_slot, Some(36));
            assert_eq!(
                events[0].stack,
                ItemStack::new(ItemKind::IronIngot, 32, None)
            );
        }

        #[test]
        fn should_drop_item_stack_set_creative_mode_slot() {
            let mut app = App::new();
            let (client_ent, mut client_helper) = scenario_single_client(&mut app);

            // Process a tick to get past the "on join" logic.
            app.update();
            client_helper.clear_sent();

            app.world.entity_mut(client_ent).insert(GameMode::Creative);

            client_helper.send(
                &valence_protocol::packet::c2s::play::CreativeInventoryActionC2s {
                    slot: -1,
                    clicked_item: Some(ItemStack::new(ItemKind::IronIngot, 32, None)),
                },
            );

            app.update();

            // Make assertions
            let events = app
                .world
                .get_resource::<Events<DropItemStack>>()
                .expect("expected drop item stack events")
                .iter_current_update_events()
                .collect::<Vec<_>>();

            assert_eq!(events.len(), 1);
            assert_eq!(events[0].client, client_ent);
            assert_eq!(events[0].from_slot, None);
            assert_eq!(
                events[0].stack,
                ItemStack::new(ItemKind::IronIngot, 32, None)
            );
        }

        #[test]
        fn should_drop_item_stack_click_container_outside() {
            let mut app = App::new();
            let (client_ent, mut client_helper) = scenario_single_client(&mut app);
            let mut cursor_item = app
                .world
                .get_mut::<CursorItem>(client_ent)
                .expect("could not find client");
            cursor_item.0 = Some(ItemStack::new(ItemKind::IronIngot, 32, None));
            let inv_state = app
                .world
                .get_mut::<ClientInventoryState>(client_ent)
                .expect("could not find client");
            let state_id = inv_state.state_id.0;

            // Process a tick to get past the "on join" logic.
            app.update();
            client_helper.clear_sent();

            client_helper.send(&valence_protocol::packet::c2s::play::ClickSlotC2s {
                window_id: 0,
                slot_idx: -999,
                button: 0,
                mode: ClickMode::Click,
                state_id: VarInt(state_id),
                slot_changes: vec![],
                carried_item: None,
            });

            app.update();

            // Make assertions
            let cursor_item = app
                .world
                .get::<CursorItem>(client_ent)
                .expect("could not find client");
            assert_eq!(cursor_item.0, None);
            let events = app
                .world
                .get_resource::<Events<DropItemStack>>()
                .expect("expected drop item stack events");
            let events = events.iter_current_update_events().collect::<Vec<_>>();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].client, client_ent);
            assert_eq!(events[0].from_slot, None);
            assert_eq!(
                events[0].stack,
                ItemStack::new(ItemKind::IronIngot, 32, None)
            );
        }

        #[test]
        fn should_drop_item_click_container_with_dropkey_single() {
            let mut app = App::new();
            let (client_ent, mut client_helper) = scenario_single_client(&mut app);
            let inv_state = app
                .world
                .get_mut::<ClientInventoryState>(client_ent)
                .expect("could not find client");
            let state_id = inv_state.state_id.0;
            let mut inventory = app
                .world
                .get_mut::<Inventory>(client_ent)
                .expect("could not find inventory");
            inventory.set_slot(40, ItemStack::new(ItemKind::IronIngot, 32, None));

            // Process a tick to get past the "on join" logic.
            app.update();
            client_helper.clear_sent();

            client_helper.send(&valence_protocol::packet::c2s::play::ClickSlotC2s {
                window_id: 0,
                slot_idx: 40,
                button: 0,
                mode: ClickMode::DropKey,
                state_id: VarInt(state_id),
                slot_changes: vec![Slot {
                    idx: 40,
                    item: Some(ItemStack::new(ItemKind::IronIngot, 31, None)),
                }],
                carried_item: None,
            });

            app.update();

            // Make assertions
            let events = app
                .world
                .get_resource::<Events<DropItemStack>>()
                .expect("expected drop item stack events");
            let events = events.iter_current_update_events().collect::<Vec<_>>();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].client, client_ent);
            assert_eq!(events[0].from_slot, Some(40));
            assert_eq!(
                events[0].stack,
                ItemStack::new(ItemKind::IronIngot, 1, None)
            );
        }

        #[test]
        fn should_drop_item_stack_click_container_with_dropkey() {
            let mut app = App::new();
            let (client_ent, mut client_helper) = scenario_single_client(&mut app);
            let inv_state = app
                .world
                .get_mut::<ClientInventoryState>(client_ent)
                .expect("could not find client");
            let state_id = inv_state.state_id.0;
            let mut inventory = app
                .world
                .get_mut::<Inventory>(client_ent)
                .expect("could not find inventory");
            inventory.set_slot(40, ItemStack::new(ItemKind::IronIngot, 32, None));

            // Process a tick to get past the "on join" logic.
            app.update();
            client_helper.clear_sent();

            client_helper.send(&valence_protocol::packet::c2s::play::ClickSlotC2s {
                window_id: 0,
                slot_idx: 40,
                button: 1, // pressing control
                mode: ClickMode::DropKey,
                state_id: VarInt(state_id),
                slot_changes: vec![Slot {
                    idx: 40,
                    item: None,
                }],
                carried_item: None,
            });

            app.update();

            // Make assertions
            let events = app
                .world
                .get_resource::<Events<DropItemStack>>()
                .expect("expected drop item stack events");
            let events = events.iter_current_update_events().collect::<Vec<_>>();
            assert_eq!(events.len(), 1);
            assert_eq!(events[0].client, client_ent);
            assert_eq!(events[0].from_slot, Some(40));
            assert_eq!(
                events[0].stack,
                ItemStack::new(ItemKind::IronIngot, 32, None)
            );
        }
    }

    #[test]
    fn dragging_items() {
        let mut app = App::new();
        let (client_ent, mut client_helper) = scenario_single_client(&mut app);
        app.world.get_mut::<CursorItem>(client_ent).unwrap().0 =
            Some(ItemStack::new(ItemKind::Diamond, 64, None));

        // Process a tick to get past the "on join" logic.
        app.update();
        client_helper.clear_sent();

        let inv_state = app.world.get::<ClientInventoryState>(client_ent).unwrap();
        let window_id = inv_state.window_id;
        let state_id = inv_state.state_id.0;

        let drag_packet = ClickSlotC2s {
            window_id,
            state_id: VarInt(state_id),
            slot_idx: -999,
            button: 2,
            mode: ClickMode::Drag,
            slot_changes: vec![
                Slot {
                    idx: 9,
                    item: Some(ItemStack::new(ItemKind::Diamond, 21, None)),
                },
                Slot {
                    idx: 10,
                    item: Some(ItemStack::new(ItemKind::Diamond, 21, None)),
                },
                Slot {
                    idx: 11,
                    item: Some(ItemStack::new(ItemKind::Diamond, 21, None)),
                },
            ],
            carried_item: Some(ItemStack::new(ItemKind::Diamond, 1, None)),
        };
        client_helper.send(&drag_packet);

        app.update();
        let sent_packets = client_helper.collect_sent();
        assert_eq!(sent_packets.len(), 0);

        let cursor_item = app
            .world
            .get::<CursorItem>(client_ent)
            .expect("could not find client");
        assert_eq!(
            cursor_item.0,
            Some(ItemStack::new(ItemKind::Diamond, 1, None))
        );
        let inventory = app
            .world
            .get::<Inventory>(client_ent)
            .expect("could not find inventory");
        for i in 9..12 {
            assert_eq!(
                inventory.slot(i),
                Some(&ItemStack::new(ItemKind::Diamond, 21, None))
            );
        }
    }
}
