use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

use crate::inventory::{
    convert_to_player_slot_id, ClickMode, ClientInventoryState, CursorItem, DropItemStackEvent,
    HeldItem, Inventory, InventoryKind, OpenInventory, SlotChange,
};
use crate::protocol::packets::play::{
    ClickSlotC2s, CloseScreenS2c, CreativeInventoryActionC2s, InventoryS2c, OpenScreenS2c,
    ScreenHandlerSlotUpdateS2c, UpdateSelectedSlotC2s,
};
use crate::protocol::VarInt;
use crate::testing::ScenarioSingleClient;
use crate::{GameMode, ItemKind, ItemStack};

#[test]
fn test_should_open_inventory() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer: _,
    } = ScenarioSingleClient::new();

    let inventory = Inventory::new(InventoryKind::Generic3x3);
    let inventory_ent = app.world.spawn(inventory).id();

    // Process a tick to get past the "on join" logic.
    app.update();
    helper.clear_received();

    // Open the inventory.
    let open_inventory = OpenInventory::new(inventory_ent);
    app.world
        .get_entity_mut(client)
        .expect("could not find client")
        .insert(open_inventory);

    app.update();

    // Make assertions
    let sent_packets = helper.collect_received();

    sent_packets.assert_count::<OpenScreenS2c>(1);
    sent_packets.assert_count::<InventoryS2c>(1);
    sent_packets.assert_order::<(OpenScreenS2c, InventoryS2c)>();
}

#[test]
fn test_should_close_inventory() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer: _,
    } = ScenarioSingleClient::new();

    let inventory = Inventory::new(InventoryKind::Generic3x3);
    let inventory_ent = app.world.spawn(inventory).id();

    // Process a tick to get past the "on join" logic.
    app.update();
    helper.clear_received();

    // Open the inventory.
    let open_inventory = OpenInventory::new(inventory_ent);
    app.world
        .get_entity_mut(client)
        .expect("could not find client")
        .insert(open_inventory);

    app.update();
    helper.clear_received();

    // Close the inventory.
    app.world
        .get_entity_mut(client)
        .expect("could not find client")
        .remove::<OpenInventory>();

    app.update();

    // Make assertions
    let sent_packets = helper.collect_received();

    sent_packets.assert_count::<CloseScreenS2c>(1);
}

#[test]
fn test_should_remove_invalid_open_inventory() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer: _,
    } = ScenarioSingleClient::new();

    let inventory = Inventory::new(InventoryKind::Generic3x3);
    let inventory_ent = app.world.spawn(inventory).id();

    // Process a tick to get past the "on join" logic.
    app.update();
    helper.clear_received();

    // Open the inventory.
    let open_inventory = OpenInventory::new(inventory_ent);
    app.world
        .get_entity_mut(client)
        .expect("could not find client")
        .insert(open_inventory);

    app.update();
    helper.clear_received();

    // Remove the inventory.
    app.world.despawn(inventory_ent);

    app.update();

    // Make assertions
    assert!(app.world.get::<OpenInventory>(client).is_none());

    let sent_packets = helper.collect_received();
    sent_packets.assert_count::<CloseScreenS2c>(1);
}

#[test]
fn test_should_modify_player_inventory_click_slot() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer: _,
    } = ScenarioSingleClient::new();

    // Process a tick to get past the "on join" logic.
    app.update();
    helper.clear_received();

    let mut inventory = app
        .world
        .get_mut::<Inventory>(client)
        .expect("could not find inventory for client");
    inventory.set_slot(20, ItemStack::new(ItemKind::Diamond, 2, None));

    // Make the client click the slot and pick up the item.
    let state_id = app
        .world
        .get::<ClientInventoryState>(client)
        .unwrap()
        .state_id();

    helper.send(&ClickSlotC2s {
        window_id: 0,
        button: 0,
        mode: ClickMode::Click,
        state_id: VarInt(state_id.0),
        slot_idx: 20,
        slot_changes: vec![SlotChange {
            idx: 20,
            item: None,
        }]
        .into(),
        carried_item: Some(ItemStack::new(ItemKind::Diamond, 2, None)),
    });

    app.update();

    // Make assertions
    let sent_packets = helper.collect_received();

    // because the inventory was changed as a result of the client's click, the
    // server should not send any packets to the client because the client
    // already knows about the change.
    sent_packets.assert_count::<InventoryS2c>(0);
    sent_packets.assert_count::<ScreenHandlerSlotUpdateS2c>(0);

    let inventory = app
        .world
        .get::<Inventory>(client)
        .expect("could not find inventory for client");

    assert_eq!(inventory.slot(20), None);

    let cursor_item = app
        .world
        .get::<CursorItem>(client)
        .expect("could not find client");

    assert_eq!(
        cursor_item.0,
        Some(ItemStack::new(ItemKind::Diamond, 2, None))
    );
}

#[test]
fn test_should_modify_player_inventory_server_side() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer: _,
    } = ScenarioSingleClient::new();

    // Process a tick to get past the "on join" logic.
    app.update();

    let mut inventory = app
        .world
        .get_mut::<Inventory>(client)
        .expect("could not find inventory for client");
    inventory.set_slot(20, ItemStack::new(ItemKind::Diamond, 2, None));

    app.update();
    helper.clear_received();

    // Modify the inventory.
    let mut inventory = app
        .world
        .get_mut::<Inventory>(client)
        .expect("could not find inventory for client");
    inventory.set_slot(21, ItemStack::new(ItemKind::IronIngot, 1, None));

    app.update();

    // Make assertions
    let sent_packets = helper.collect_received();
    // because the inventory was modified server side, the client needs to be
    // updated with the change.
    sent_packets.assert_count::<ScreenHandlerSlotUpdateS2c>(1);
}

#[test]
fn test_should_sync_entire_player_inventory() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer: _,
    } = ScenarioSingleClient::new();

    // Process a tick to get past the "on join" logic.
    app.update();
    helper.clear_received();

    let mut inventory = app
        .world
        .get_mut::<Inventory>(client)
        .expect("could not find inventory for client");
    inventory.changed = u64::MAX;

    app.update();

    // Make assertions
    let sent_packets = helper.collect_received();
    sent_packets.assert_count::<InventoryS2c>(1);
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
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer: _,
    } = ScenarioSingleClient::new();

    let inventory_ent = set_up_open_inventory(&mut app, client);

    let mut inventory = app
        .world
        .get_mut::<Inventory>(inventory_ent)
        .expect("could not find inventory for client");

    inventory.set_slot(20, ItemStack::new(ItemKind::Diamond, 2, None));

    // Process a tick to get past the "on join" logic.
    app.update();
    helper.clear_received();

    // Make the client click the slot and pick up the item.
    let inv_state = app.world.get::<ClientInventoryState>(client).unwrap();
    let state_id = inv_state.state_id();
    let window_id = inv_state.window_id();
    helper.send(&ClickSlotC2s {
        window_id,
        state_id: VarInt(state_id.0),
        slot_idx: 20,
        button: 0,
        mode: ClickMode::Click,
        slot_changes: vec![SlotChange {
            idx: 20,
            item: None,
        }]
        .into(),
        carried_item: Some(ItemStack::new(ItemKind::Diamond, 2, None)),
    });

    app.update();

    // Make assertions
    let sent_packets = helper.collect_received();

    // because the inventory was modified as a result of the client's click, the
    // server should not send any packets to the client because the client
    // already knows about the change.
    sent_packets.assert_count::<InventoryS2c>(0);
    sent_packets.assert_count::<ScreenHandlerSlotUpdateS2c>(0);

    let inventory = app
        .world
        .get::<Inventory>(inventory_ent)
        .expect("could not find inventory");
    assert_eq!(inventory.slot(20), None);
    let cursor_item = app
        .world
        .get::<CursorItem>(client)
        .expect("could not find client");
    assert_eq!(
        cursor_item.0,
        Some(ItemStack::new(ItemKind::Diamond, 2, None))
    );
}

#[test]
fn test_should_modify_open_inventory_server_side() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer: _,
    } = ScenarioSingleClient::new();

    let inventory_ent = set_up_open_inventory(&mut app, client);

    // Process a tick to get past the "on join" logic.
    app.update();
    helper.clear_received();

    // Modify the inventory.
    let mut inventory = app
        .world
        .get_mut::<Inventory>(inventory_ent)
        .expect("could not find inventory for client");
    inventory.set_slot(5, ItemStack::new(ItemKind::IronIngot, 1, None));

    app.update();

    // Make assertions
    let sent_packets = helper.collect_received();

    // because the inventory was modified server side, the client needs to be
    // updated with the change.
    sent_packets.assert_count::<ScreenHandlerSlotUpdateS2c>(1);

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
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer: _,
    } = ScenarioSingleClient::new();

    let inventory_ent = set_up_open_inventory(&mut app, client);

    // Process a tick to get past the "on join" logic.
    app.update();
    helper.clear_received();

    let mut inventory = app
        .world
        .get_mut::<Inventory>(inventory_ent)
        .expect("could not find inventory");
    inventory.changed = u64::MAX;

    app.update();

    // Make assertions
    let sent_packets = helper.collect_received();
    sent_packets.assert_count::<InventoryS2c>(1);
}

#[test]
fn test_set_creative_mode_slot_handling() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer: _,
    } = ScenarioSingleClient::new();

    let mut game_mode = app
        .world
        .get_mut::<GameMode>(client)
        .expect("could not find client");
    *game_mode.as_mut() = GameMode::Creative;

    // Process a tick to get past the "on join" logic.
    app.update();
    helper.clear_received();

    helper.send(&CreativeInventoryActionC2s {
        slot: 36,
        clicked_item: Some(ItemStack::new(ItemKind::Diamond, 2, None)),
    });

    app.update();

    // Make assertions
    let inventory = app
        .world
        .get::<Inventory>(client)
        .expect("could not find inventory for client");

    assert_eq!(
        inventory.slot(36),
        Some(&ItemStack::new(ItemKind::Diamond, 2, None))
    );
}

#[test]
fn test_ignore_set_creative_mode_slot_if_not_creative() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer: _,
    } = ScenarioSingleClient::new();

    let mut game_mode = app
        .world
        .get_mut::<GameMode>(client)
        .expect("could not find client");
    *game_mode.as_mut() = GameMode::Survival;

    // Process a tick to get past the "on join" logic.
    app.update();
    helper.clear_received();

    helper.send(&CreativeInventoryActionC2s {
        slot: 36,
        clicked_item: Some(ItemStack::new(ItemKind::Diamond, 2, None)),
    });

    app.update();

    // Make assertions
    let inventory = app
        .world
        .get::<Inventory>(client)
        .expect("could not find inventory for client");
    assert_eq!(inventory.slot(36), None);
}

#[test]
fn test_window_id_increments() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer: _,
    } = ScenarioSingleClient::new();

    let inventory = Inventory::new(InventoryKind::Generic9x3);
    let inventory_ent = app.world.spawn(inventory).id();

    // Process a tick to get past the "on join" logic.
    app.update();
    helper.clear_received();

    for _ in 0..3 {
        let open_inventory = OpenInventory::new(inventory_ent);
        app.world
            .get_entity_mut(client)
            .expect("could not find client")
            .insert(open_inventory);

        app.update();

        app.world
            .get_entity_mut(client)
            .expect("could not find client")
            .remove::<OpenInventory>();

        app.update();
    }

    // Make assertions
    let inv_state = app
        .world
        .get::<ClientInventoryState>(client)
        .expect("could not find client");
    assert_eq!(inv_state.window_id(), 3);
}

#[test]
fn test_should_handle_set_held_item() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer: _,
    } = ScenarioSingleClient::new();

    // Process a tick to get past the "on join" logic.
    app.update();
    helper.clear_received();

    helper.send(&UpdateSelectedSlotC2s { slot: 4 });

    app.update();

    // Make assertions
    let held = app
        .world
        .get::<HeldItem>(client)
        .expect("could not find client");

    assert_eq!(held.slot(), 40);
}

#[test]
fn should_not_increment_state_id_on_cursor_item_change() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer: _,
    } = ScenarioSingleClient::new();

    // Process a tick to get past the "on join" logic.
    app.update();
    helper.clear_received();

    let inv_state = app
        .world
        .get::<ClientInventoryState>(client)
        .expect("could not find client");
    let expected_state_id = inv_state.state_id().0;

    let mut cursor_item = app.world.get_mut::<CursorItem>(client).unwrap();
    cursor_item.0 = Some(ItemStack::new(ItemKind::Diamond, 2, None));

    app.update();

    // Make assertions
    let inv_state = app
        .world
        .get::<ClientInventoryState>(client)
        .expect("could not find client");
    assert_eq!(
        inv_state.state_id().0,
        expected_state_id,
        "state id should not have changed"
    );
}

mod dropping_items {
    use super::*;
    use crate::inventory::{convert_to_player_slot_id, PlayerAction};
    use crate::protocol::packets::play::PlayerActionC2s;
    use crate::{BlockPos, Direction};

    #[test]
    fn should_drop_item_player_action() {
        let ScenarioSingleClient {
            mut app,
            client,
            mut helper,
            layer: _,
        } = ScenarioSingleClient::new();

        // Process a tick to get past the "on join" logic.
        app.update();
        helper.clear_received();

        let mut inventory = app
            .world
            .get_mut::<Inventory>(client)
            .expect("could not find inventory");
        inventory.set_slot(36, ItemStack::new(ItemKind::IronIngot, 3, None));

        helper.send(&PlayerActionC2s {
            action: PlayerAction::DropItem,
            position: BlockPos::new(0, 0, 0),
            direction: Direction::Down,
            sequence: VarInt(0),
        });

        app.update();

        // Make assertions
        let inventory = app
            .world
            .get::<Inventory>(client)
            .expect("could not find client");

        assert_eq!(
            inventory.slot(36),
            Some(&ItemStack::new(ItemKind::IronIngot, 2, None))
        );

        let events = app
            .world
            .get_resource::<Events<DropItemStackEvent>>()
            .expect("expected drop item stack events");

        let events = events.iter_current_update_events().collect::<Vec<_>>();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].client, client);
        assert_eq!(events[0].from_slot, Some(36));
        assert_eq!(
            events[0].stack,
            ItemStack::new(ItemKind::IronIngot, 1, None)
        );

        let sent_packets = helper.collect_received();

        sent_packets.assert_count::<ScreenHandlerSlotUpdateS2c>(0);
    }

    #[test]
    fn should_drop_item_stack_player_action() {
        let ScenarioSingleClient {
            mut app,
            client,
            mut helper,
            layer: _,
        } = ScenarioSingleClient::new();

        // Process a tick to get past the "on join" logic.
        app.update();
        helper.clear_received();

        let mut inventory = app
            .world
            .get_mut::<Inventory>(client)
            .expect("could not find inventory");
        inventory.set_slot(36, ItemStack::new(ItemKind::IronIngot, 32, None));

        helper.send(&PlayerActionC2s {
            action: PlayerAction::DropAllItems,
            position: BlockPos::new(0, 0, 0),
            direction: Direction::Down,
            sequence: VarInt(0),
        });

        app.update();

        // Make assertions
        let held = app
            .world
            .get::<HeldItem>(client)
            .expect("could not find client");
        assert_eq!(held.slot(), 36);
        let inventory = app
            .world
            .get::<Inventory>(client)
            .expect("could not find inventory");
        assert_eq!(inventory.slot(36), None);
        let events = app
            .world
            .get_resource::<Events<DropItemStackEvent>>()
            .expect("expected drop item stack events");
        let events = events.iter_current_update_events().collect::<Vec<_>>();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].client, client);
        assert_eq!(events[0].from_slot, Some(36));
        assert_eq!(
            events[0].stack,
            ItemStack::new(ItemKind::IronIngot, 32, None)
        );
    }

    #[test]
    fn should_drop_item_stack_set_creative_mode_slot() {
        let ScenarioSingleClient {
            mut app,
            client,
            mut helper,
            layer: _,
        } = ScenarioSingleClient::new();

        // Process a tick to get past the "on join" logic.
        app.update();
        helper.clear_received();

        app.world.entity_mut(client).insert(GameMode::Creative);

        helper.send(&CreativeInventoryActionC2s {
            slot: -1,
            clicked_item: Some(ItemStack::new(ItemKind::IronIngot, 32, None)),
        });

        app.update();

        // Make assertions
        let events = app
            .world
            .get_resource::<Events<DropItemStackEvent>>()
            .expect("expected drop item stack events")
            .iter_current_update_events()
            .collect::<Vec<_>>();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].client, client);
        assert_eq!(events[0].from_slot, None);
        assert_eq!(
            events[0].stack,
            ItemStack::new(ItemKind::IronIngot, 32, None)
        );
    }

    #[test]
    fn should_drop_item_stack_click_container_outside() {
        let ScenarioSingleClient {
            mut app,
            client,
            mut helper,
            layer: _,
        } = ScenarioSingleClient::new();

        // Process a tick to get past the "on join" logic.
        app.update();
        helper.clear_received();

        let mut cursor_item = app
            .world
            .get_mut::<CursorItem>(client)
            .expect("could not find client");
        cursor_item.0 = Some(ItemStack::new(ItemKind::IronIngot, 32, None));
        let inv_state = app
            .world
            .get_mut::<ClientInventoryState>(client)
            .expect("could not find client");
        let state_id = inv_state.state_id().0;

        helper.send(&ClickSlotC2s {
            window_id: 0,
            state_id: VarInt(state_id),
            slot_idx: -999,
            button: 0,
            mode: ClickMode::Click,
            slot_changes: vec![].into(),
            carried_item: None,
        });

        app.update();

        // Make assertions
        let cursor_item = app
            .world
            .get::<CursorItem>(client)
            .expect("could not find client");

        assert_eq!(cursor_item.0, None);

        let events = app
            .world
            .get_resource::<Events<DropItemStackEvent>>()
            .expect("expected drop item stack events");

        let events = events.iter_current_update_events().collect::<Vec<_>>();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].client, client);
        assert_eq!(events[0].from_slot, None);
        assert_eq!(
            events[0].stack,
            ItemStack::new(ItemKind::IronIngot, 32, None)
        );
    }

    #[test]
    fn should_drop_item_click_container_with_dropkey_single() {
        let ScenarioSingleClient {
            mut app,
            client,
            mut helper,
            layer: _,
        } = ScenarioSingleClient::new();

        // Process a tick to get past the "on join" logic.
        app.update();
        helper.clear_received();

        let inv_state = app
            .world
            .get_mut::<ClientInventoryState>(client)
            .expect("could not find client");

        let state_id = inv_state.state_id().0;

        let mut inventory = app
            .world
            .get_mut::<Inventory>(client)
            .expect("could not find inventory");

        inventory.set_slot(40, ItemStack::new(ItemKind::IronIngot, 32, None));

        helper.send(&ClickSlotC2s {
            window_id: 0,
            slot_idx: 40,
            button: 0,
            mode: ClickMode::DropKey,
            state_id: VarInt(state_id),
            slot_changes: vec![SlotChange {
                idx: 40,
                item: Some(ItemStack::new(ItemKind::IronIngot, 31, None)),
            }]
            .into(),
            carried_item: None,
        });

        app.update();

        // Make assertions
        let events = app
            .world
            .get_resource::<Events<DropItemStackEvent>>()
            .expect("expected drop item stack events");

        let events = events.iter_current_update_events().collect::<Vec<_>>();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].client, client);
        assert_eq!(events[0].from_slot, Some(40));
        assert_eq!(
            events[0].stack,
            ItemStack::new(ItemKind::IronIngot, 1, None)
        );
    }

    #[test]
    fn should_drop_item_stack_click_container_with_dropkey() {
        let ScenarioSingleClient {
            mut app,
            client,
            mut helper,
            layer: _,
        } = ScenarioSingleClient::new();

        // Process a tick to get past the "on join" logic.
        app.update();
        helper.clear_received();

        let inv_state = app
            .world
            .get_mut::<ClientInventoryState>(client)
            .expect("could not find client");

        let state_id = inv_state.state_id().0;

        let mut inventory = app
            .world
            .get_mut::<Inventory>(client)
            .expect("could not find inventory");

        inventory.set_slot(40, ItemStack::new(ItemKind::IronIngot, 32, None));

        helper.send(&ClickSlotC2s {
            window_id: 0,
            slot_idx: 40,
            button: 1, // pressing control
            mode: ClickMode::DropKey,
            state_id: VarInt(state_id),
            slot_changes: vec![SlotChange {
                idx: 40,
                item: None,
            }]
            .into(),
            carried_item: None,
        });

        app.update();

        // Make assertions
        let events = app
            .world
            .get_resource::<Events<DropItemStackEvent>>()
            .expect("expected drop item stack events");

        let events = events.iter_current_update_events().collect::<Vec<_>>();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].client, client);
        assert_eq!(events[0].from_slot, Some(40));
        assert_eq!(
            events[0].stack,
            ItemStack::new(ItemKind::IronIngot, 32, None)
        );
    }

    /// The item should be dropped successfully, if the player has an inventory
    /// open and the slot id points to his inventory.
    #[test]
    fn should_drop_item_player_open_inventory_with_dropkey() {
        let ScenarioSingleClient {
            mut app,
            client,
            mut helper,
            layer: _,
        } = ScenarioSingleClient::new();

        // Process a tick to get past the "on join" logic.
        app.update();

        let mut inventory = app
            .world
            .get_mut::<Inventory>(client)
            .expect("could not find inventory");

        inventory.set_slot(
            convert_to_player_slot_id(InventoryKind::Generic9x3, 50),
            ItemStack::new(ItemKind::IronIngot, 32, None),
        );

        let _inventory_ent = set_up_open_inventory(&mut app, client);

        app.update();

        helper.clear_received();

        let inv_state = app
            .world
            .get_mut::<ClientInventoryState>(client)
            .expect("could not find client");

        let state_id = inv_state.state_id().0;
        let window_id = inv_state.window_id();

        helper.send(&ClickSlotC2s {
            window_id,
            state_id: VarInt(state_id),
            slot_idx: 50, // not pressing control
            button: 0,
            mode: ClickMode::DropKey,
            slot_changes: vec![SlotChange {
                idx: 50,
                item: Some(ItemStack::new(ItemKind::IronIngot, 31, None)),
            }]
            .into(),
            carried_item: None,
        });

        app.update();

        // Make assertions
        let events = app
            .world
            .get_resource::<Events<DropItemStackEvent>>()
            .expect("expected drop item stack events");

        let player_inventory = app
            .world
            .get::<Inventory>(client)
            .expect("could not find inventory");

        let events = events.iter_current_update_events().collect::<Vec<_>>();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].client, client);
        assert_eq!(
            events[0].from_slot,
            Some(convert_to_player_slot_id(InventoryKind::Generic9x3, 50))
        );

        assert_eq!(
            events[0].stack,
            ItemStack::new(ItemKind::IronIngot, 1, None)
        );

        // Also make sure that the player inventory was updated correctly.
        let expected_player_slot_id = convert_to_player_slot_id(InventoryKind::Generic9x3, 50);
        assert_eq!(
            player_inventory.slot(expected_player_slot_id),
            Some(&ItemStack::new(ItemKind::IronIngot, 31, None))
        );
    }
}

/// The item stack should be dropped successfully, if the player has an
/// inventory open and the slot id points to his inventory.
#[test]
fn should_drop_item_stack_player_open_inventory_with_dropkey() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer: _,
    } = ScenarioSingleClient::new();

    // Process a tick to get past the "on join" logic.
    app.update();

    let mut inventory = app
        .world
        .get_mut::<Inventory>(client)
        .expect("could not find inventory");

    inventory.set_slot(
        convert_to_player_slot_id(InventoryKind::Generic9x3, 50),
        ItemStack::new(ItemKind::IronIngot, 32, None),
    );

    let _inventory_ent = set_up_open_inventory(&mut app, client);

    app.update();
    helper.clear_received();

    let inv_state = app
        .world
        .get_mut::<ClientInventoryState>(client)
        .expect("could not find client");

    let state_id = inv_state.state_id().0;
    let window_id = inv_state.window_id();

    helper.send(&ClickSlotC2s {
        window_id,
        state_id: VarInt(state_id),
        slot_idx: 50, // pressing control, the whole stack is dropped
        button: 1,
        mode: ClickMode::DropKey,
        slot_changes: vec![SlotChange {
            idx: 50,
            item: None,
        }]
        .into(),
        carried_item: None,
    });

    app.update();

    // Make assertions
    let events = app
        .world
        .get_resource::<Events<DropItemStackEvent>>()
        .expect("expected drop item stack events");

    let player_inventory = app
        .world
        .get::<Inventory>(client)
        .expect("could not find inventory");

    let events = events.iter_current_update_events().collect::<Vec<_>>();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].client, client);
    assert_eq!(
        events[0].from_slot,
        Some(convert_to_player_slot_id(InventoryKind::Generic9x3, 50))
    );
    assert_eq!(
        events[0].stack,
        ItemStack::new(ItemKind::IronIngot, 32, None)
    );

    // Also make sure that the player inventory was updated correctly.
    let expected_player_slot_id = convert_to_player_slot_id(InventoryKind::Generic9x3, 50);
    assert_eq!(player_inventory.slot(expected_player_slot_id), None);
}

#[test]
fn dragging_items() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer: _,
    } = ScenarioSingleClient::new();

    // Process a tick to get past the "on join" logic.
    app.update();
    helper.clear_received();

    app.world.get_mut::<CursorItem>(client).unwrap().0 =
        Some(ItemStack::new(ItemKind::Diamond, 64, None));

    let inv_state = app.world.get::<ClientInventoryState>(client).unwrap();
    let window_id = inv_state.window_id();
    let state_id = inv_state.state_id().0;

    let drag_packet = ClickSlotC2s {
        window_id,
        state_id: VarInt(state_id),
        slot_idx: -999,
        button: 2,
        mode: ClickMode::Drag,
        slot_changes: vec![
            SlotChange {
                idx: 9,
                item: Some(ItemStack::new(ItemKind::Diamond, 21, None)),
            },
            SlotChange {
                idx: 10,
                item: Some(ItemStack::new(ItemKind::Diamond, 21, None)),
            },
            SlotChange {
                idx: 11,
                item: Some(ItemStack::new(ItemKind::Diamond, 21, None)),
            },
        ]
        .into(),
        carried_item: Some(ItemStack::new(ItemKind::Diamond, 1, None)),
    };
    helper.send(&drag_packet);

    app.update();
    let sent_packets = helper.collect_received();
    assert_eq!(sent_packets.0.len(), 0);

    let cursor_item = app
        .world
        .get::<CursorItem>(client)
        .expect("could not find client");

    assert_eq!(
        cursor_item.0,
        Some(ItemStack::new(ItemKind::Diamond, 1, None))
    );

    let inventory = app
        .world
        .get::<Inventory>(client)
        .expect("could not find inventory");

    for i in 9..12 {
        assert_eq!(
            inventory.slot(i),
            Some(&ItemStack::new(ItemKind::Diamond, 21, None))
        );
    }
}
