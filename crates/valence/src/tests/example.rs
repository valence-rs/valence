//! Examples of valence unit tests that need to test the behavior of the server,
//! and not just the logic of a single function. This module is meant to be a
//! pallette of examples for how to write such tests, with various levels of
//! complexity.
//!
//! Some of the tests in this file may be inferior duplicates of real tests.

use bevy_app::App;
use valence_core::packet::c2s::play::PositionAndOnGround;
use valence_core::packet::s2c::play::S2cPlayPacket;

use super::*;
use crate::prelude::*;

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
    let mut app = App::new();
    let (client_ent, mut client_helper) = scenario_single_client(&mut app);

    // Send a packet as the client to the server.
    let packet = PositionAndOnGround {
        position: DVec3::new(12.0, 64.0, 0.0),
        on_ground: true,
    };
    client_helper.send(&packet);

    // Process the packet.
    app.update();

    // Make assertions
    let pos = app.world.get::<Position>(client_ent).unwrap();
    assert_eq!(pos.0, DVec3::new(12.0, 64.0, 0.0));
}

/// A unit test where we want to test what packets are sent to the client.
#[test]
fn example_test_open_inventory() {
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
    app.world
        .get::<Client>(client_ent)
        .expect("client not found");
    let sent_packets = client_helper.collect_sent();

    assert_packet_count!(sent_packets, 1, S2cPlayPacket::OpenScreenS2c(_));
    assert_packet_count!(sent_packets, 1, S2cPlayPacket::InventoryS2c(_));
    assert_packet_order!(
        sent_packets,
        S2cPlayPacket::OpenScreenS2c(_),
        S2cPlayPacket::InventoryS2c(_)
    );
}
