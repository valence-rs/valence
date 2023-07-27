use bevy_app::App;
use valence_client::packet::GameStateChangeS2c;
use valence_client::weather::{Rain, Thunder};
use valence_client::Client;
use valence_instance::Instance;

use crate::testing::{scenario_single_client, PacketFrames};

#[test]
fn test_weather_instance() {
    let mut app = App::new();
    let (_, mut client_helper) = scenario_single_client(&mut app);

    // Process a tick to get past the "on join" logic.
    app.update();
    client_helper.clear_received();

    // Get the instance entity.
    let instance_ent = app
        .world
        .iter_entities()
        .find(|e| e.contains::<Instance>())
        .expect("could not find instance")
        .id();

    // Insert a rain component to the instance.
    app.world.entity_mut(instance_ent).insert(Rain(0.5));
    for _ in 0..2 {
        app.update();
    }

    // Alter a rain component of the instance.
    app.world.entity_mut(instance_ent).insert(Rain(1.0));
    app.update();

    // Insert a thunder component to the instance.
    app.world.entity_mut(instance_ent).insert(Thunder(0.5));
    app.update();

    // Alter a thunder component of the instance.
    app.world.entity_mut(instance_ent).insert(Thunder(1.0));
    app.update();

    // Remove the rain component from the instance.
    app.world.entity_mut(instance_ent).remove::<Rain>();
    for _ in 0..2 {
        app.update();
    }

    // Make assertions.
    let sent_packets = client_helper.collect_received();

    assert_weather_packets(sent_packets);
}

#[test]
fn test_weather_client() {
    let mut app = App::new();
    let (_, mut client_helper) = scenario_single_client(&mut app);

    // Process a tick to get past the "on join" logic.
    app.update();
    client_helper.clear_received();

    // Get the client entity.
    let client_ent = app
        .world
        .iter_entities()
        .find(|e| e.contains::<Client>())
        .expect("could not find client")
        .id();

    // Insert a rain component to the client.
    app.world.entity_mut(client_ent).insert(Rain(0.5));
    for _ in 0..2 {
        app.update();
    }

    // Alter a rain component of the client.
    app.world.entity_mut(client_ent).insert(Rain(1.0));
    app.update();

    // Insert a thunder component to the client.
    app.world.entity_mut(client_ent).insert(Thunder(0.5));
    app.update();

    // Alter a thunder component of the client.
    app.world.entity_mut(client_ent).insert(Thunder(1.0));
    app.update();

    // Remove the rain component from the client.
    app.world.entity_mut(client_ent).remove::<Rain>();
    for _ in 0..2 {
        app.update();
    }

    // Make assertions.
    let sent_packets = client_helper.collect_received();

    assert_weather_packets(sent_packets);
}

#[track_caller]
fn assert_weather_packets(sent_packets: PacketFrames) {
    sent_packets.assert_count::<GameStateChangeS2c>(6);
}
