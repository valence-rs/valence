//! Examples of valence unit tests that need to test the behavior of the server,
//! and not just the logic of a single function. This module is meant to be a
//! pallette of examples for how to write such tests, with various levels of
//! complexity.
//!
//! Some of the tests in this file may be inferior duplicates of real tests.

use bevy_app::App;
use glam::DVec3;
use valence_client::movement::PositionAndOnGroundC2s;
use valence_client::Client;
use valence_core::Server;
use valence_entity::Position;
use valence_inventory::packet::{InventoryS2c, OpenScreenS2c};
use valence_inventory::{Inventory, InventoryKind, OpenInventory};

use crate::testing::ScenarioSingleClient;
use crate::DefaultPlugins;

/// The server's tick should increment every update.
#[test]
fn example_test_server_tick_increment() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins);

    let tick = app.world.resource::<Server>().current_tick();

    app.update();

    let server = app.world.resource::<Server>();
    assert_eq!(server.current_tick(), tick + 1);
}

/// A unit test where we want to test what happens when a client sends a
/// packet to the server.
#[test]
fn example_test_client_position() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer: _,
    } = ScenarioSingleClient::new();

    // Send a packet as the client to the server.
    let packet = PositionAndOnGroundC2s {
        position: DVec3::new(12.0, 64.0, 0.0),
        on_ground: true,
    };
    helper.send(&packet);

    // Process the packet.
    app.update();

    // Make assertions
    let pos = app.world.get::<Position>(client).unwrap();
    assert_eq!(pos.0, DVec3::new(12.0, 64.0, 0.0));
}

/// A unit test where we want to test what packets are sent to the client.
#[test]
fn example_test_open_inventory() {
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
    app.update();

    // Make assertions
    app.world.get::<Client>(client).expect("client not found");

    let sent_packets = helper.collect_received();

    sent_packets.assert_count::<OpenScreenS2c>(1);
    sent_packets.assert_count::<InventoryS2c>(1);

    sent_packets.assert_order::<(OpenScreenS2c, InventoryS2c)>();
}
