use std::iter::FusedIterator;

use bevy_ecs::prelude::*;
use tracing::{debug, warn};
use valence_protocol::packets::s2c::play::{
    CloseContainerS2c, OpenScreen, SetContainerContentEncode, SetContainerSlotEncode,
};
use valence_protocol::types::{GameMode, WindowType};
use valence_protocol::{ItemStack, Text, VarInt};

use crate::client::event::{ClickContainer, CloseContainer, SetCreativeModeSlot};
use crate::client::Client;

#[derive(Debug, Clone, Component)]
pub struct Inventory {
    title: Text,
    kind: InventoryKind,
    slots: Box<[Option<ItemStack>]>,
    /// Contains a set bit for each modified slot in `slots`.
    modified: u64,
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
        }
    }

    #[track_caller]
    pub fn slot(&self, idx: u16) -> Option<&ItemStack> {
        self.slots
            .get(idx as usize)
            .expect("slot index out of range")
            .as_ref()
    }

    #[track_caller]
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

    #[track_caller]
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

    fn slot_slice(&self) -> &[Option<ItemStack>] {
        self.slots.as_ref()
    }
}

/// Send updates for each client's player inventory.
pub(crate) fn update_player_inventories(
    mut query: Query<(&mut Inventory, &mut Client), Without<OpenInventory>>,
) {
    for (mut inventory, mut client) in query.iter_mut() {
        if inventory.kind != InventoryKind::Player {
            warn!("Inventory on client entity is not a player inventory");
        }

        if inventory.modified != 0 {
            if inventory.modified == u64::MAX {
                // Update the whole inventory.
                client.inventory_state_id += 1;
                let cursor_item = client.cursor_item.clone();
                let state_id = client.inventory_state_id.0;
                client.write_packet(&SetContainerContentEncode {
                    window_id: 0,
                    state_id: VarInt(state_id),
                    slots: inventory.slot_slice(),
                    carried_item: &cursor_item,
                });

                client.cursor_item_modified = false;
            } else {
                // send the modified slots

                // The slots that were NOT modified by this client, and they need to be sent
                let modified_filtered = inventory.modified & !client.inventory_slots_modified;
                if modified_filtered != 0 {
                    client.inventory_state_id += 1;
                    let state_id = client.inventory_state_id.0;
                    for (i, slot) in inventory.slots.iter().enumerate() {
                        if ((modified_filtered >> i) & 1) == 1 {
                            client.write_packet(&SetContainerSlotEncode {
                                window_id: 0,
                                state_id: VarInt(state_id),
                                slot_idx: i as i16,
                                slot_data: slot.as_ref(),
                            });
                        }
                    }
                }
            }

            inventory.modified = 0;
            client.inventory_slots_modified = 0;
        }

        if client.cursor_item_modified {
            client.inventory_state_id += 1;

            client.cursor_item_modified = false;

            let cursor_item = client.cursor_item.clone();
            let state_id = client.inventory_state_id.0;
            client.write_packet(&SetContainerSlotEncode {
                window_id: -1,
                state_id: VarInt(state_id),
                slot_idx: -1,
                slot_data: cursor_item.as_ref(),
            });
        }
    }
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

/// Used to indicate that the client with this component is currently viewing
/// an inventory.
#[derive(Debug, Clone, Component)]
pub struct OpenInventory {
    /// The Entity with the `Inventory` component that the client is currently
    /// viewing.
    pub(crate) entity: Entity,
    client_modified: u64,
}

impl OpenInventory {
    pub fn new(entity: Entity) -> Self {
        OpenInventory {
            entity,
            client_modified: 0,
        }
    }

    pub fn entity(&self) -> Entity {
        self.entity
    }
}

/// Handles the `OpenInventory` component being added to a client, which
/// indicates that the client is now viewing an inventory, and sends inventory
/// updates to the client when the inventory is modified.
pub(crate) fn update_open_inventories(
    mut commands: Commands,
    mut clients: Query<(Entity, &mut Client, &mut OpenInventory)>,
    mut inventories: Query<&mut Inventory>,
) {
    // These operations need to happen in this order.

    // send the inventory contents to all clients that are viewing an inventory
    for (client_entity, mut client, mut open_inventory) in clients.iter_mut() {
        // validate that the inventory exists
        let Ok(inventory) = inventories.get_component::<Inventory>(open_inventory.entity) else {
            // the inventory no longer exists, so close the inventory
            commands.entity(client_entity).remove::<OpenInventory>();
            continue;
        };

        if open_inventory.is_added() {
            // send the inventory to the client if the client just opened the inventory
            client.window_id = client.window_id % 100 + 1;
            open_inventory.client_modified = 0;

            let packet = OpenScreen {
                window_id: VarInt(client.window_id.into()),
                window_type: WindowType::from(inventory.kind),
                window_title: inventory.title.clone(),
            };
            client.write_packet(&packet);

            let packet = SetContainerContentEncode {
                window_id: client.window_id,
                state_id: VarInt(client.inventory_state_id.0),
                slots: inventory.slot_slice(),
                carried_item: &client.cursor_item.clone(),
            };
            client.write_packet(&packet);
        } else {
            // the client is already viewing the inventory
            if inventory.modified == u64::MAX {
                // send the entire inventory
                client.inventory_state_id += 1;
                let packet = SetContainerContentEncode {
                    window_id: client.window_id,
                    state_id: VarInt(client.inventory_state_id.0),
                    slots: inventory.slot_slice(),
                    carried_item: &client.cursor_item.clone(),
                };
                client.write_packet(&packet);
            } else {
                // send the modified slots
                let window_id = client.window_id as i8;
                // The slots that were NOT modified by this client, and they need to be sent
                let modified_filtered = inventory.modified & !open_inventory.client_modified;
                if modified_filtered != 0 {
                    client.inventory_state_id += 1;
                    let state_id = client.inventory_state_id.0;
                    for (i, slot) in inventory.slots.iter().enumerate() {
                        if (modified_filtered >> i) & 1 == 1 {
                            client.write_packet(&SetContainerSlotEncode {
                                window_id,
                                state_id: VarInt(state_id),
                                slot_idx: i as i16,
                                slot_data: slot.as_ref(),
                            });
                        }
                    }
                }
            }
        }

        open_inventory.client_modified = 0;
        client.inventory_slots_modified = 0;
    }

    // reset the modified flag
    for (_, _, open_inventory) in clients.iter_mut() {
        // validate that the inventory exists
        if let Ok(mut inventory) = inventories.get_component_mut::<Inventory>(open_inventory.entity)
        {
            inventory.modified = 0;
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

pub(crate) fn handle_click_container(
    mut clients: Query<(&mut Client, &mut Inventory, Option<&mut OpenInventory>)>,
    mut inventories: Query<&mut Inventory, Without<Client>>,
    mut events: EventReader<ClickContainer>,
) {
    for event in events.iter() {
        let Ok((mut client, mut client_inventory, mut open_inventory)) =
            clients.get_mut(event.client) else {
                // the client does not exist, ignore
                continue;
            };

        // validate the window id
        if (event.window_id == 0) != open_inventory.is_none() {
            warn!(
                "Client sent a click with an invalid window id for current state: window_id = {}, \
                 open_inventory present = {}",
                event.window_id,
                open_inventory.is_some()
            );
            continue;
        }

        if let Some(open_inventory) = open_inventory.as_mut() {
            // the player is interacting with an inventory that is open
            let Ok(mut target_inventory) = inventories.get_component_mut::<Inventory>(open_inventory.entity) else {
                // the inventory does not exist, ignore
                continue;
            };
            if client.inventory_state_id.0 != event.state_id {
                // client is out of sync, resync, ignore click
                debug!("Client state id mismatch, resyncing");
                client.inventory_state_id += 1;
                let packet = SetContainerContentEncode {
                    window_id: client.window_id,
                    state_id: VarInt(client.inventory_state_id.0),
                    slots: target_inventory.slot_slice(),
                    carried_item: &client.cursor_item.clone(),
                };
                client.write_packet(&packet);
                continue;
            }

            client.cursor_item = event.carried_item.clone();

            for (slot_id, item) in event.slot_changes.clone() {
                if (0i16..target_inventory.slot_count() as i16).contains(&slot_id) {
                    // the client is interacting with a slot in the target inventory
                    target_inventory.replace_slot(slot_id as u16, item);
                    open_inventory.client_modified |= 1 << slot_id;
                } else {
                    // the client is interacting with a slot in their own inventory
                    let slot_id = convert_to_player_slot_id(target_inventory.kind, slot_id as u16);
                    client_inventory.replace_slot(slot_id, item);
                    client.inventory_slots_modified |= 1 << slot_id;
                }
            }
        } else {
            // the client is interacting with their own inventory

            if client.inventory_state_id.0 != event.state_id {
                // client is out of sync, resync, and ignore the click
                debug!("Client state id mismatch, resyncing");
                client.inventory_state_id += 1;
                let packet = SetContainerContentEncode {
                    window_id: client.window_id,
                    state_id: VarInt(client.inventory_state_id.0),
                    slots: client_inventory.slot_slice(),
                    carried_item: &client.cursor_item.clone(),
                };
                client.write_packet(&packet);
                continue;
            }

            // TODO: do more validation on the click
            client.cursor_item = event.carried_item.clone();
            for (slot_id, item) in event.slot_changes.clone() {
                if (0i16..client_inventory.slot_count() as i16).contains(&slot_id) {
                    client_inventory.replace_slot(slot_id as u16, item);
                    client.inventory_slots_modified |= 1 << slot_id;
                } else {
                    // the client is trying to interact with a slot that does not exist,
                    // ignore
                    warn!(
                        "Client attempted to interact with slot {} which does not exist",
                        slot_id
                    );
                }
            }
        }
    }
}

pub(crate) fn handle_set_slot_creative(
    mut clients: Query<(&mut Client, &mut Inventory)>,
    mut events: EventReader<SetCreativeModeSlot>,
) {
    for event in events.iter() {
        if let Ok((mut client, mut inventory)) = clients.get_mut(event.client) {
            if client.game_mode() != GameMode::Creative {
                // the client is not in creative mode, ignore
                continue;
            }
            inventory.replace_slot(event.slot as u16, event.clicked_item.clone());
            inventory.modified &= !(1 << event.slot); // clear the modified bit, since we are about to send the update
            client.inventory_state_id += 1;
            let state_id = client.inventory_state_id.0;
            // HACK: notchian clients rely on the server to send the slot update when in
            // creative mode Simply marking the slot as modified is not enough. This was
            // discovered because shift-clicking the destroy item slot in creative mode does
            // not work without this hack.
            client.write_packet(&SetContainerSlotEncode {
                window_id: 0,
                state_id: VarInt(state_id),
                slot_idx: event.slot,
                slot_data: event.clicked_item.as_ref(),
            });
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

#[cfg(test)]
mod test {
    use bevy_app::App;
    use valence_protocol::packets::S2cPlayPacket;
    use valence_protocol::ItemKind;

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
    fn test_should_open_inventory() -> anyhow::Result<()> {
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
        app.update();

        // Make assertions
        let sent_packets = client_helper.collect_sent()?;

        assert_packet_count!(sent_packets, 1, S2cPlayPacket::OpenScreen(_));
        assert_packet_count!(sent_packets, 1, S2cPlayPacket::SetContainerContent(_));
        assert_packet_order!(
            sent_packets,
            S2cPlayPacket::OpenScreen(_),
            S2cPlayPacket::SetContainerContent(_)
        );

        Ok(())
    }

    #[test]
    fn test_should_close_inventory() -> anyhow::Result<()> {
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
        app.update();
        client_helper.clear_sent();

        // Close the inventory.
        app.world
            .get_entity_mut(client_ent)
            .expect("could not find client")
            .remove::<OpenInventory>();

        app.update();
        app.update();

        // Make assertions
        let sent_packets = client_helper.collect_sent()?;

        assert_packet_count!(sent_packets, 1, S2cPlayPacket::CloseContainerS2c(_));

        Ok(())
    }

    #[test]
    fn test_should_remove_invalid_open_inventory() -> anyhow::Result<()> {
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
        app.update();
        client_helper.clear_sent();

        // Remove the inventory.
        app.world.despawn(inventory_ent);

        app.update();

        // Make assertions
        assert!(app.world.get::<OpenInventory>(client_ent).is_none());
        let sent_packets = client_helper.collect_sent()?;
        assert_packet_count!(sent_packets, 1, S2cPlayPacket::CloseContainerS2c(_));

        Ok(())
    }

    #[test]
    fn test_should_modify_player_inventory_click_container() -> anyhow::Result<()> {
        let mut app = App::new();
        let (client_ent, mut client_helper) = scenario_single_client(&mut app);
        let mut inventory = app
            .world
            .get_mut::<Inventory>(client_ent)
            .expect("could not find inventory for client");
        inventory.replace_slot(20, ItemStack::new(ItemKind::Diamond, 2, None));

        // Process a tick to get past the "on join" logic.
        app.update();
        client_helper.clear_sent();

        // Modify the inventory.
        let state_id = app
            .world
            .get::<Client>(client_ent)
            .unwrap()
            .inventory_state_id;
        client_helper.send(&valence_protocol::packets::c2s::play::ClickContainer {
            window_id: 0,
            button: 0,
            mode: valence_protocol::types::ClickContainerMode::Click,
            state_id: VarInt(state_id.0),
            slot_idx: 20,
            slots: vec![(20, None)],
            carried_item: Some(ItemStack::new(ItemKind::Diamond, 1, None)),
        });

        app.update();
        app.update();

        // Make assertions
        let sent_packets = client_helper.collect_sent()?;

        // because the inventory was modified as a result of the client's click, the
        // server should not send any packets to the client because the client
        // already knows about the change.
        assert_packet_count!(
            sent_packets,
            0,
            S2cPlayPacket::SetContainerContent(_) | S2cPlayPacket::SetContainerSlot(_)
        );

        Ok(())
    }

    #[test]
    fn test_should_modify_player_inventory_server_side() -> anyhow::Result<()> {
        let mut app = App::new();
        let (client_ent, mut client_helper) = scenario_single_client(&mut app);
        let mut inventory = app
            .world
            .get_mut::<Inventory>(client_ent)
            .expect("could not find inventory for client");
        inventory.replace_slot(20, ItemStack::new(ItemKind::Diamond, 2, None));

        // Process a tick to get past the "on join" logic.
        app.update();
        client_helper.clear_sent();

        // Modify the inventory.
        let mut inventory = app
            .world
            .get_mut::<Inventory>(client_ent)
            .expect("could not find inventory for client");
        inventory.replace_slot(21, ItemStack::new(ItemKind::IronIngot, 1, None));
        assert_eq!(inventory.slot(21).unwrap().item, ItemKind::IronIngot);

        app.update();
        app.update();

        // Make assertions
        let sent_packets = client_helper.collect_sent()?;
        for packet in &sent_packets {
            println!("{:?}", packet);
        }
        // because the inventory was modified server side, the client needs to be
        // updated with the change.
        assert_packet_count!(sent_packets, 1, S2cPlayPacket::SetContainerSlot(_));

        Ok(())
    }
}
