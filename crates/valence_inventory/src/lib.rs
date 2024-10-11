#![doc = include_str!("../README.md")]

use std::borrow::Cow;
use std::iter::FusedIterator;
use std::num::Wrapping;
use std::ops::Range;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use derive_more::{Deref, DerefMut};
use player_inventory::PlayerInventory;
use tracing::{debug, warn};
use valence_server::client::{Client, FlushPacketsSet, SpawnClientsSet};
use valence_server::event_loop::{EventLoopPreUpdate, PacketEvent};
pub use valence_server::protocol::packets::play::click_slot_c2s::{ClickMode, SlotChange};
use valence_server::protocol::packets::play::open_screen_s2c::WindowType;
pub use valence_server::protocol::packets::play::player_action_c2s::PlayerAction;
use valence_server::protocol::packets::play::{
    ClickSlotC2s, CloseHandledScreenC2s, CloseScreenS2c, CreativeInventoryActionC2s, InventoryS2c,
    OpenScreenS2c, PlayerActionC2s, ScreenHandlerSlotUpdateS2c, UpdateSelectedSlotC2s,
    UpdateSelectedSlotS2c,
};
use valence_server::protocol::{VarInt, WritePacket};
use valence_server::text::IntoText;
use valence_server::{GameMode, ItemKind, ItemStack, Text};

pub mod player_inventory;
mod validate;

pub struct InventoryPlugin;

impl Plugin for InventoryPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            PreUpdate,
            init_new_client_inventories.after(SpawnClientsSet),
        )
        .add_systems(
            PostUpdate,
            (
                update_client_on_close_inventory.before(update_open_inventories),
                update_player_selected_slot,
                update_open_inventories,
                update_player_inventories,
                update_cursor_item,
            )
                .before(FlushPacketsSet),
        )
        .add_systems(
            EventLoopPreUpdate,
            (
                handle_update_selected_slot,
                handle_click_slot,
                handle_creative_inventory_action,
                handle_close_handled_screen,
                handle_player_actions,
            ),
        )
        .init_resource::<InventorySettings>()
        .add_event::<ClickSlotEvent>()
        .add_event::<DropItemStackEvent>()
        .add_event::<CreativeInventoryActionEvent>()
        .add_event::<UpdateSelectedSlotEvent>();
    }
}

#[derive(Debug, Clone, Component)]
pub struct Inventory {
    title: Text,
    kind: InventoryKind,
    slots: Box<[ItemStack]>,
    /// Contains a set bit for each modified slot in `slots`.
    #[doc(hidden)]
    pub changed: u64,
    /// Makes an inventory read-only for clients. This will prevent adding
    /// or removing items. If this is a player inventory
    /// This will also make it impossible to drop items while not
    /// in the inventory (e.g. by pressing Q)
    pub readonly: bool,
}

impl Inventory {
    pub fn new(kind: InventoryKind) -> Self {
        // TODO: default title to the correct translation key instead
        Self::with_title(kind, "Inventory")
    }

    pub fn with_title<'a, T: IntoText<'a>>(kind: InventoryKind, title: T) -> Self {
        Inventory {
            title: title.into_cow_text().into_owned(),
            kind,
            slots: vec![ItemStack::EMPTY; kind.slot_count()].into(),
            changed: 0,
            readonly: false,
        }
    }

    #[track_caller]
    pub fn slot(&self, idx: u16) -> &ItemStack {
        self.slots
            .get(idx as usize)
            .expect("slot index out of range")
    }

    /// Sets the slot at the given index to the given item stack.
    ///
    /// See also [`Inventory::replace_slot`].
    ///
    /// ```
    /// # use valence_inventory::*;
    /// # use valence_server::item::{ItemStack, ItemKind};
    /// let mut inv = Inventory::new(InventoryKind::Generic9x1);
    /// inv.set_slot(0, ItemStack::new(ItemKind::Diamond, 1, None));
    /// assert_eq!(inv.slot(0).item, ItemKind::Diamond);
    /// ```
    #[track_caller]
    #[inline]
    pub fn set_slot<I: Into<ItemStack>>(&mut self, idx: u16, item: I) {
        let _ = self.replace_slot(idx, item);
    }

    /// Replaces the slot at the given index with the given item stack, and
    /// returns the old stack in that slot.
    ///
    /// See also [`Inventory::set_slot`].
    ///
    /// ```
    /// # use valence_inventory::*;
    /// # use valence_server::item::{ItemStack, ItemKind};
    /// let mut inv = Inventory::new(InventoryKind::Generic9x1);
    /// inv.set_slot(0, ItemStack::new(ItemKind::Diamond, 1, None));
    /// let old = inv.replace_slot(0, ItemStack::new(ItemKind::IronIngot, 1, None));
    /// assert_eq!(old.item, ItemKind::Diamond);
    /// ```
    #[track_caller]
    #[must_use]
    pub fn replace_slot<I: Into<ItemStack>>(&mut self, idx: u16, item: I) -> ItemStack {
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
    /// # use valence_inventory::*;
    /// # use valence_server::item::{ItemStack, ItemKind};
    /// let mut inv = Inventory::new(InventoryKind::Generic9x1);
    /// inv.set_slot(0, ItemStack::new(ItemKind::Diamond, 1, None));
    /// assert!(inv.slot(1).is_empty());
    /// inv.swap_slot(0, 1);
    /// assert_eq!(inv.slot(1).item, ItemKind::Diamond);
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
    /// # use valence_inventory::*;
    /// # use valence_server::item::{ItemStack, ItemKind};
    /// let mut inv = Inventory::new(InventoryKind::Generic9x1);
    /// inv.set_slot(0, ItemStack::new(ItemKind::Diamond, 1, None));
    /// inv.set_slot_amount(0, 64);
    /// assert_eq!(inv.slot(0).count, 64);
    /// ```
    #[track_caller]
    pub fn set_slot_amount(&mut self, idx: u16, amount: i8) {
        assert!(idx < self.slot_count(), "slot index out of range");

        let item = &mut self.slots[idx as usize];

        if !item.is_empty() {
            if item.count == amount {
                return;
            }
            item.count = amount;
            self.changed |= 1 << idx;
        }
    }

    pub fn slot_count(&self) -> u16 {
        self.slots.len() as u16
    }

    pub fn slots(
        &self,
    ) -> impl ExactSizeIterator<Item = &ItemStack> + DoubleEndedIterator + FusedIterator + Clone + '_
    {
        self.slots.iter()
    }

    pub fn kind(&self) -> InventoryKind {
        self.kind
    }

    /// The text displayed on the inventory's title bar.
    ///
    /// ```
    /// # use valence_inventory::*;
    /// # use valence_server::item::{ItemStack, ItemKind};
    /// # use valence_server::text::Text;
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
    /// # use valence_inventory::*;
    /// let mut inv = Inventory::new(InventoryKind::Generic9x3);
    /// inv.set_title("Box of Holding");
    /// ```
    #[inline]
    pub fn set_title<'a, T: IntoText<'a>>(&mut self, title: T) {
        let _ = self.replace_title(title);
    }

    /// Replace the text displayed on the inventory's title bar, and returns the
    /// old text.
    #[must_use]
    pub fn replace_title<'a, T: IntoText<'a>>(&mut self, title: T) -> Text {
        // TODO: set title modified flag
        std::mem::replace(&mut self.title, title.into_cow_text().into_owned())
    }

    pub(crate) fn slot_slice(&self) -> &[ItemStack] {
        &self.slots
    }

    /// Returns the first empty slot in the given range, or `None` if there are
    /// no empty slots in the range.
    ///
    /// ```
    /// # use valence_inventory::*;
    /// # use valence_server::item::*;
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

        range.find(|&idx| self.slots[idx as usize].is_empty())
    }

    /// Returns the first empty slot in the inventory, or `None` if there are no
    /// empty slots.
    /// ```
    /// # use valence_inventory::*;
    /// # use valence_server::item::*;
    /// let mut inv = Inventory::new(InventoryKind::Generic9x1);
    /// inv.set_slot(0, ItemStack::new(ItemKind::Diamond, 1, None));
    /// inv.set_slot(2, ItemStack::new(ItemKind::GoldIngot, 1, None));
    /// inv.set_slot(3, ItemStack::new(ItemKind::IronIngot, 1, None));
    /// assert_eq!(inv.first_empty_slot(), Some(1));
    /// ```
    #[inline]
    pub fn first_empty_slot(&self) -> Option<u16> {
        self.first_empty_slot_in(0..self.slot_count())
    }

    /// Returns the first slot with the given [`ItemKind`] in the inventory
    /// where `count < stack_max`, or `None` if there are no empty slots.
    /// ```
    /// # use valence_inventory::*;
    /// # use valence_server::item::*;
    /// let mut inv = Inventory::new(InventoryKind::Generic9x1);
    /// inv.set_slot(0, ItemStack::new(ItemKind::Diamond, 1, None));
    /// inv.set_slot(2, ItemStack::new(ItemKind::GoldIngot, 64, None));
    /// inv.set_slot(3, ItemStack::new(ItemKind::IronIngot, 1, None));
    /// inv.set_slot(4, ItemStack::new(ItemKind::GoldIngot, 1, None));
    /// assert_eq!(
    ///     inv.first_slot_with_item_in(ItemKind::GoldIngot, 64, 0..5),
    ///     Some(4)
    /// );
    /// ```
    pub fn first_slot_with_item_in(
        &self,
        item: ItemKind,
        stack_max: i8,
        mut range: Range<u16>,
    ) -> Option<u16> {
        assert!(
            (0..=self.slot_count()).contains(&range.start)
                && (0..=self.slot_count()).contains(&range.end),
            "slot range out of range"
        );
        assert!(stack_max > 0, "stack_max must be greater than 0");

        range.find(|&idx| {
            let stack = &self.slots[idx as usize];
            stack.item == item && stack.count < stack_max
        })
    }

    /// Returns the first slot with the given [`ItemKind`] in the inventory
    /// where `count < stack_max`, or `None` if there are no empty slots.
    /// ```
    /// # use valence_inventory::*;
    /// # use valence_server::item::*;
    /// let mut inv = Inventory::new(InventoryKind::Generic9x1);
    /// inv.set_slot(0, ItemStack::new(ItemKind::Diamond, 1, None));
    /// inv.set_slot(2, ItemStack::new(ItemKind::GoldIngot, 64, None));
    /// inv.set_slot(3, ItemStack::new(ItemKind::IronIngot, 1, None));
    /// inv.set_slot(4, ItemStack::new(ItemKind::GoldIngot, 1, None));
    /// assert_eq!(inv.first_slot_with_item(ItemKind::GoldIngot, 64), Some(4));
    /// ```
    #[inline]
    pub fn first_slot_with_item(&self, item: ItemKind, stack_max: i8) -> Option<u16> {
        self.first_slot_with_item_in(item, stack_max, 0..self.slot_count())
    }
}

/// Miscellaneous inventory data.
#[derive(Component, Debug)]
pub struct ClientInventoryState {
    /// The current window ID. Incremented when inventories are opened.
    window_id: u8,
    state_id: Wrapping<i32>,
    /// Tracks what slots have been changed by this client in this tick, so we
    /// don't need to send updates for them.
    slots_changed: u64,
    /// If `Some`: The item the user thinks they updated their cursor item to on
    /// the last tick.
    /// If `None`: the user did not update their cursor item in the last tick.
    /// This is so we can inform the user of the update through change detection
    /// when they differ in a given tick
    client_updated_cursor_item: Option<ItemStack>,
}

impl ClientInventoryState {
    #[doc(hidden)]
    pub fn window_id(&self) -> u8 {
        self.window_id
    }

    #[doc(hidden)]
    pub fn state_id(&self) -> Wrapping<i32> {
        self.state_id
    }
}

/// Indicates which hotbar slot the player is currently holding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Component, Deref)]
pub struct HeldItem {
    held_item_slot: u16,
}

impl HeldItem {
    /// The slot ID of the currently held item, in the range 36-44 inclusive.
    /// This value is safe to use on the player's inventory directly.
    pub fn slot(&self) -> u16 {
        self.held_item_slot
    }

    pub fn hotbar_idx(&self) -> u8 {
        PlayerInventory::slot_to_hotbar(self.held_item_slot)
    }

    pub fn set_slot(&mut self, slot: u16) {
        // temp
        assert!(
            PlayerInventory::SLOTS_HOTBAR.contains(&slot),
            "slot index of {slot} out of bounds"
        );

        self.held_item_slot = slot;
    }

    pub fn set_hotbar_idx(&mut self, hotbar_idx: u8) {
        self.set_slot(PlayerInventory::hotbar_to_slot(hotbar_idx))
    }
}

/// The item stack that the client thinks it's holding under the mouse
/// cursor.
#[derive(Component, Clone, PartialEq, Default, Debug, Deref, DerefMut)]
pub struct CursorItem(pub ItemStack);

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
/// # use valence_inventory::*;
/// # use valence_server::item::*;
/// let mut player_inventory = Inventory::new(InventoryKind::Player);
/// player_inventory.set_slot(36, ItemStack::new(ItemKind::Diamond, 1, None));
///
/// let target_inventory = Inventory::new(InventoryKind::Generic9x3);
/// let window = InventoryWindow::new(&player_inventory, Some(&target_inventory));
///
/// assert_eq!(window.slot(54), &ItemStack::new(ItemKind::Diamond, 1, None));
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
    pub fn slot(&self, idx: u16) -> &ItemStack {
        if let Some(open_inv) = self.open_inventory.as_ref() {
            if idx < open_inv.slot_count() {
                open_inv.slot(idx)
            } else {
                self.player_inventory
                    .slot(convert_to_player_slot_id(open_inv.kind(), idx))
            }
        } else {
            self.player_inventory.slot(idx)
        }
    }

    #[track_caller]
    pub fn slot_count(&self) -> u16 {
        if let Some(open_inv) = &self.open_inventory {
            // when the window is split, we can only access the main slots of player's
            // inventory
            PlayerInventory::MAIN_SIZE + open_inv.slot_count()
        } else {
            self.player_inventory.slot_count()
        }
    }
}

/// A helper to represent the inventory window that the player is currently
/// viewing. Handles dispatching reads/writes to the correct inventory.
///
/// This is a writable version of [`InventoryWindow`].
///
/// ```
/// # use valence_inventory::*;
/// # use valence_server::item::*;
/// let mut player_inventory = Inventory::new(InventoryKind::Player);
/// let mut target_inventory = Inventory::new(InventoryKind::Generic9x3);
/// let mut window = InventoryWindowMut::new(&mut player_inventory, Some(&mut target_inventory));
///
/// window.set_slot(54, ItemStack::new(ItemKind::Diamond, 1, None));
///
/// assert_eq!(
///     player_inventory.slot(36),
///     &ItemStack::new(ItemKind::Diamond, 1, None)
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
    pub fn slot(&self, idx: u16) -> &ItemStack {
        if let Some(open_inv) = self.open_inventory.as_ref() {
            if idx < open_inv.slot_count() {
                open_inv.slot(idx)
            } else {
                self.player_inventory
                    .slot(convert_to_player_slot_id(open_inv.kind(), idx))
            }
        } else {
            self.player_inventory.slot(idx)
        }
    }

    #[track_caller]
    #[must_use]
    pub fn replace_slot<I: Into<ItemStack>>(&mut self, idx: u16, item: I) -> ItemStack {
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
    pub fn set_slot<I: Into<ItemStack>>(&mut self, idx: u16, item: I) {
        let _ = self.replace_slot(idx, item);
    }

    pub fn slot_count(&self) -> u16 {
        if let Some(open_inv) = &self.open_inventory {
            // when the window is split, we can only access the main slots of player's
            // inventory
            PlayerInventory::MAIN_SIZE + open_inv.slot_count()
        } else {
            self.player_inventory.slot_count()
        }
    }
}

/// Attach the necessary inventory components to new clients.
fn init_new_client_inventories(clients: Query<Entity, Added<Client>>, mut commands: Commands) {
    for entity in &clients {
        commands.entity(entity).insert((
            Inventory::new(InventoryKind::Player),
            CursorItem(ItemStack::EMPTY),
            ClientInventoryState {
                window_id: 0,
                state_id: Wrapping(0),
                slots_changed: 0,
                client_updated_cursor_item: None,
            },
            HeldItem {
                // First slot of the hotbar.
                held_item_slot: 36,
            },
        ));
    }
}

/// Send updates for each client's player inventory.
fn update_player_inventories(
    mut query: Query<
        (
            &mut Inventory,
            &mut Client,
            &mut ClientInventoryState,
            &CursorItem,
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
        let Ok(inventory) = inventories.get_mut(open_inventory.entity) else {
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
        // Since these happen every gametick we only want to trigger change detection
        // if we actually did update these. Otherwise systems that are
        // running looking for changes to the `Inventory`,`ClientInventoryState`
        // or `OpenInventory` components get unneccerely ran each gametick
        open_inventory
            .map_unchanged(|f| &mut f.client_changed)
            .set_if_neq(0);
        inv_state
            .map_unchanged(|f| &mut f.slots_changed)
            .set_if_neq(0);
        inventory.map_unchanged(|f| &mut f.changed).set_if_neq(0);
    }
}

fn update_cursor_item(
    mut clients: Query<(&mut Client, &mut ClientInventoryState, &CursorItem), Changed<CursorItem>>,
) {
    for (mut client, inv_state, cursor_item) in &mut clients {
        // The cursor item was not the item the user themselves interacted with
        if inv_state.client_updated_cursor_item.as_ref() != Some(&cursor_item.0) {
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

        inv_state
            .map_unchanged(|f| &mut f.client_updated_cursor_item)
            .set_if_neq(None);
    }
}

/// Handles clients telling the server that they are closing an inventory.
fn handle_close_handled_screen(mut packets: EventReader<PacketEvent>, mut commands: Commands) {
    for packet in packets.read() {
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
    for entity in &mut removals.read() {
        if let Ok((mut client, inv_state)) = clients.get_mut(entity) {
            client.write_packet(&CloseScreenS2c {
                window_id: inv_state.window_id,
            })
        }
    }
}

// TODO: make this event user friendly.
#[derive(Event, Clone, Debug)]
pub struct ClickSlotEvent {
    pub client: Entity,
    pub window_id: u8,
    pub state_id: i32,
    pub slot_id: i16,
    pub button: i8,
    pub mode: ClickMode,
    pub slot_changes: Vec<SlotChange>,
    pub carried_item: ItemStack,
}

#[derive(Event, Clone, Debug)]
pub struct DropItemStackEvent {
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
    mut drop_item_stack_events: EventWriter<DropItemStackEvent>,
    mut click_slot_events: EventWriter<ClickSlotEvent>,
) {
    for packet in packets.read() {
        let Some(pkt) = packet.decode::<ClickSlotC2s>() else {
            // Not the packet we're looking for.
            continue;
        };

        let Ok((mut client, mut client_inv, mut inv_state, open_inventory, mut cursor_item)) =
            clients.get_mut(packet.client)
        else {
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

            let stack = std::mem::take(&mut cursor_item.0);

            if !stack.is_empty() {
                drop_item_stack_events.send(DropItemStackEvent {
                    client: packet.client,
                    from_slot: None,
                    stack,
                });
            }
        } else if pkt.mode == ClickMode::DropKey {
            // The client is dropping an item by pressing the drop key.

            let entire_stack = pkt.button == 1;

            // Needs to open the inventory for if the player is dropping an item while
            // having an inventory open.
            if let Some(open_inventory) = open_inventory {
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

                if (0_i16..target_inventory.slot_count() as i16).contains(&pkt.slot_idx) {
                    // The player is dropping an item from another inventory.

                    if target_inventory.readonly {
                        // resync target inventory
                        client.write_packet(&InventoryS2c {
                            window_id: inv_state.window_id,
                            state_id: VarInt(inv_state.state_id.0),
                            slots: Cow::Borrowed(target_inventory.slot_slice()),
                            carried_item: Cow::Borrowed(&cursor_item.0),
                        });
                        continue;
                    }

                    let stack = target_inventory.slot(pkt.slot_idx as u16);

                    if !stack.is_empty() {
                        let dropped = if entire_stack || stack.count == 1 {
                            target_inventory.replace_slot(pkt.slot_idx as u16, ItemStack::EMPTY)
                        } else {
                            let stack = stack.clone().with_count(stack.count - 1);
                            let mut old_slot =
                                target_inventory.replace_slot(pkt.slot_idx as u16, stack);
                            // we already checked that the slot was not empty and that the
                            // stack count is > 1
                            old_slot.count = 1;
                            old_slot
                        };

                        drop_item_stack_events.send(DropItemStackEvent {
                            client: packet.client,
                            from_slot: Some(pkt.slot_idx as u16),
                            stack: dropped,
                        });
                    }
                } else {
                    // The player is dropping an item from their inventory.

                    if client_inv.readonly {
                        // resync the client inventory
                        client.write_packet(&InventoryS2c {
                            window_id: 0,
                            state_id: VarInt(inv_state.state_id.0),
                            slots: Cow::Borrowed(client_inv.slot_slice()),
                            carried_item: Cow::Borrowed(&cursor_item.0),
                        });
                        continue;
                    }

                    let slot_id =
                        convert_to_player_slot_id(target_inventory.kind, pkt.slot_idx as u16);

                    let stack = client_inv.slot(slot_id);

                    if !stack.is_empty() {
                        let dropped = if entire_stack || stack.count == 1 {
                            client_inv.replace_slot(slot_id, ItemStack::EMPTY)
                        } else {
                            let stack = stack.clone().with_count(stack.count - 1);
                            let mut old_slot = client_inv.replace_slot(slot_id, stack);
                            // we already checked that the slot was not empty and that the
                            // stack count is > 1
                            old_slot.count = 1;
                            old_slot
                        };

                        drop_item_stack_events.send(DropItemStackEvent {
                            client: packet.client,
                            from_slot: Some(slot_id),
                            stack: dropped,
                        });
                    }
                }
            } else {
                // The player has no inventory open and is dropping an item from their
                // inventory.

                if client_inv.readonly {
                    // resync the client inventory
                    client.write_packet(&InventoryS2c {
                        window_id: 0,
                        state_id: VarInt(inv_state.state_id.0),
                        slots: Cow::Borrowed(client_inv.slot_slice()),
                        carried_item: Cow::Borrowed(&cursor_item.0),
                    });
                    continue;
                }

                let stack = client_inv.slot(pkt.slot_idx as u16);

                if !stack.is_empty() {
                    let dropped = if entire_stack || stack.count == 1 {
                        client_inv.replace_slot(pkt.slot_idx as u16, ItemStack::EMPTY)
                    } else {
                        let stack = stack.clone().with_count(stack.count - 1);
                        let mut old_slot = client_inv.replace_slot(pkt.slot_idx as u16, stack);
                        // we already checked that the slot was not empty and that the
                        // stack count is > 1
                        old_slot.count = 1;
                        old_slot
                    };

                    drop_item_stack_events.send(DropItemStackEvent {
                        client: packet.client,
                        from_slot: Some(pkt.slot_idx as u16),
                        stack: dropped,
                    });
                }
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
                // The player is interacting with an inventory that is
                // open or has an inventory open while interacting with their own inventory.

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

                let mut new_cursor = pkt.carried_item.clone();

                for slot in pkt.slot_changes.iter() {
                    let transferred_between_inventories =
                        ((0_i16..target_inventory.slot_count() as i16).contains(&pkt.slot_idx)
                            && pkt.mode == ClickMode::Hotbar)
                            || pkt.mode == ClickMode::ShiftClick;

                    if (0_i16..target_inventory.slot_count() as i16).contains(&slot.idx) {
                        if (client_inv.readonly && transferred_between_inventories)
                            || target_inventory.readonly
                        {
                            new_cursor = cursor_item.0.clone();
                            continue;
                        }

                        target_inventory.set_slot(slot.idx as u16, slot.stack.clone());
                        open_inventory.client_changed |= 1 << slot.idx;
                    } else {
                        if (target_inventory.readonly && transferred_between_inventories)
                            || client_inv.readonly
                        {
                            new_cursor = cursor_item.0.clone();
                            continue;
                        }

                        // The client is interacting with a slot in their own inventory.
                        let slot_id =
                            convert_to_player_slot_id(target_inventory.kind, slot.idx as u16);
                        client_inv.set_slot(slot_id, slot.stack.clone());
                        inv_state.slots_changed |= 1 << slot_id;
                    }
                }

                cursor_item.set_if_neq(CursorItem(new_cursor.clone()));
                inv_state.client_updated_cursor_item = Some(new_cursor);

                if target_inventory.readonly || client_inv.readonly {
                    // resync the target inventory
                    client.write_packet(&InventoryS2c {
                        window_id: inv_state.window_id,
                        state_id: VarInt(inv_state.state_id.0),
                        slots: Cow::Borrowed(target_inventory.slot_slice()),
                        carried_item: Cow::Borrowed(&cursor_item.0),
                    });

                    // resync the client inventory
                    client.write_packet(&InventoryS2c {
                        window_id: 0,
                        state_id: VarInt(inv_state.state_id.0),
                        slots: Cow::Borrowed(client_inv.slot_slice()),
                        carried_item: Cow::Borrowed(&cursor_item.0),
                    });
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

                let mut new_cursor = pkt.carried_item.clone();

                for slot in pkt.slot_changes.iter() {
                    if (0_i16..client_inv.slot_count() as i16).contains(&slot.idx) {
                        if client_inv.readonly {
                            new_cursor = cursor_item.0.clone();
                            continue;
                        }
                        client_inv.set_slot(slot.idx as u16, slot.stack.clone());
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

                cursor_item.set_if_neq(CursorItem(new_cursor.clone()));
                inv_state.client_updated_cursor_item = Some(new_cursor);

                if client_inv.readonly {
                    // resync the client inventory
                    client.write_packet(&InventoryS2c {
                        window_id: 0,
                        state_id: VarInt(inv_state.state_id.0),
                        slots: Cow::Borrowed(client_inv.slot_slice()),
                        carried_item: Cow::Borrowed(&cursor_item.0),
                    });
                }
            }

            click_slot_events.send(ClickSlotEvent {
                client: packet.client,
                window_id: pkt.window_id,
                state_id: pkt.state_id.0,
                slot_id: pkt.slot_idx,
                button: pkt.button,
                mode: pkt.mode,
                slot_changes: pkt.slot_changes.into(),
                carried_item: pkt.carried_item,
            });
        }
    }
}

fn handle_player_actions(
    mut packets: EventReader<PacketEvent>,
    mut clients: Query<(
        &mut Inventory,
        &mut ClientInventoryState,
        &HeldItem,
        &mut Client,
    )>,
    mut drop_item_stack_events: EventWriter<DropItemStackEvent>,
) {
    for packet in packets.read() {
        if let Some(pkt) = packet.decode::<PlayerActionC2s>() {
            match pkt.action {
                PlayerAction::DropAllItems => {
                    if let Ok((mut inv, mut inv_state, &held, mut client)) =
                        clients.get_mut(packet.client)
                    {
                        if inv.readonly {
                            // resync the client inventory
                            client.write_packet(&InventoryS2c {
                                window_id: 0,
                                state_id: VarInt(inv_state.state_id.0),
                                slots: Cow::Borrowed(inv.slot_slice()),
                                carried_item: Cow::Borrowed(&ItemStack::EMPTY),
                            });
                            continue;
                        }

                        let stack = inv.replace_slot(held.slot(), ItemStack::EMPTY);

                        if !stack.is_empty() {
                            inv_state.slots_changed |= 1 << held.slot();

                            drop_item_stack_events.send(DropItemStackEvent {
                                client: packet.client,
                                from_slot: Some(held.slot()),
                                stack,
                            });
                        }
                    }
                }
                PlayerAction::DropItem => {
                    if let Ok((mut inv, mut inv_state, held, mut client)) =
                        clients.get_mut(packet.client)
                    {
                        if inv.readonly {
                            // resync the client inventory
                            client.write_packet(&InventoryS2c {
                                window_id: 0,
                                state_id: VarInt(inv_state.state_id.0),
                                slots: Cow::Borrowed(inv.slot_slice()),
                                carried_item: Cow::Borrowed(&ItemStack::EMPTY),
                            });
                            continue;
                        }

                        let mut stack = inv.replace_slot(held.slot(), ItemStack::EMPTY);

                        if !stack.is_empty() {
                            if stack.count > 1 {
                                inv.set_slot(
                                    held.slot(),
                                    stack.clone().with_count(stack.count - 1),
                                );

                                stack.count = 1;
                            }

                            inv_state.slots_changed |= 1 << held.slot();

                            drop_item_stack_events.send(DropItemStackEvent {
                                client: packet.client,
                                from_slot: Some(held.slot()),
                                stack,
                            });
                        }
                    }
                }
                PlayerAction::SwapItemWithOffhand => {
                    if let Ok((mut inv, inv_state, held, mut client)) =
                        clients.get_mut(packet.client)
                    {
                        // this check here might not actually be necessary
                        if inv.readonly {
                            // resync the client inventory
                            client.write_packet(&InventoryS2c {
                                window_id: 0,
                                state_id: VarInt(inv_state.state_id.0),
                                slots: Cow::Borrowed(inv.slot_slice()),
                                carried_item: Cow::Borrowed(&ItemStack::EMPTY),
                            });
                            continue;
                        }

                        inv.swap_slot(held.slot(), PlayerInventory::SLOT_OFFHAND);
                    }
                }
                _ => {}
            }
        }
    }
}

// TODO: make this event user friendly.
#[derive(Event, Clone, Debug)]
pub struct CreativeInventoryActionEvent {
    pub client: Entity,
    pub slot: i16,
    pub clicked_item: ItemStack,
}

fn handle_creative_inventory_action(
    mut packets: EventReader<PacketEvent>,
    mut clients: Query<(
        &mut Client,
        &mut Inventory,
        &mut ClientInventoryState,
        &GameMode,
    )>,
    mut inv_action_events: EventWriter<CreativeInventoryActionEvent>,
    mut drop_item_stack_events: EventWriter<DropItemStackEvent>,
) {
    for packet in packets.read() {
        if let Some(pkt) = packet.decode::<CreativeInventoryActionC2s>() {
            let Ok((mut client, mut inventory, mut inv_state, game_mode)) =
                clients.get_mut(packet.client)
            else {
                continue;
            };

            if *game_mode != GameMode::Creative {
                // The client is not in creative mode, ignore.
                continue;
            }

            if pkt.slot == -1 {
                let stack = pkt.clicked_item.clone();

                if !stack.is_empty() {
                    drop_item_stack_events.send(DropItemStackEvent {
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

            inv_action_events.send(CreativeInventoryActionEvent {
                client: packet.client,
                slot: pkt.slot,
                clicked_item: pkt.clicked_item,
            });
        }
    }
}

#[derive(Event, Clone, Debug)]
pub struct UpdateSelectedSlotEvent {
    pub client: Entity,
    pub slot: u8,
}

/// Handles the `HeldItem` component being changed on a client entity, which
/// indicates that the server has changed the selected hotbar slot.
fn update_player_selected_slot(mut clients: Query<(&mut Client, &HeldItem), Changed<HeldItem>>) {
    for (mut client, held_item) in &mut clients {
        client.write_packet(&UpdateSelectedSlotS2c {
            slot: held_item.hotbar_idx(),
        });
    }
}

/// Client to Server `HeldItem` Slot
fn handle_update_selected_slot(
    mut packets: EventReader<PacketEvent>,
    mut clients: Query<&mut HeldItem>,
    mut events: EventWriter<UpdateSelectedSlotEvent>,
) {
    for packet in packets.read() {
        if let Some(pkt) = packet.decode::<UpdateSelectedSlotC2s>() {
            if let Ok(mut mut_held) = clients.get_mut(packet.client) {
                let held = mut_held.bypass_change_detection();
                if pkt.slot > 8 {
                    // The client is trying to interact with a slot that does not exist, ignore.
                    continue;
                }

                held.set_hotbar_idx(pkt.slot as u8);

                events.send(UpdateSelectedSlotEvent {
                    client: packet.client,
                    slot: pkt.slot as u8,
                });
            }
        }
    }
}

/// Convert a slot that is outside a target inventory's range to a slot that is
/// inside the player's inventory.
#[doc(hidden)]
pub fn convert_to_player_slot_id(target_kind: InventoryKind, slot_id: u16) -> u16 {
    // the first slot in the player's general inventory
    let offset = target_kind.slot_count() as u16;
    slot_id - offset + 9
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
mod tests {
    use super::*;

    #[test]
    fn test_convert_to_player_slot() {
        assert_eq!(convert_to_player_slot_id(InventoryKind::Generic9x3, 27), 9);
        assert_eq!(convert_to_player_slot_id(InventoryKind::Generic9x3, 36), 18);
        assert_eq!(convert_to_player_slot_id(InventoryKind::Generic9x3, 54), 36);
        assert_eq!(convert_to_player_slot_id(InventoryKind::Generic9x1, 9), 9);
    }

    #[test]
    fn test_convert_hotbar_slot_id() {
        assert_eq!(PlayerInventory::hotbar_to_slot(0), 36);
        assert_eq!(PlayerInventory::hotbar_to_slot(4), 40);
        assert_eq!(PlayerInventory::hotbar_to_slot(8), 44);
    }
}
