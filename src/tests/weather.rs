/*
use bevy_app::App;
use valence_client::packet::GameStateChangeS2c;
use valence_client::weather::{Rain, Thunder};
use valence_client::Client;
use valence_instance::Instance;

use crate::testing::{PacketFrames, ScenarioSingleClient};

#[test]
fn test_weather_instance() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer,
    } = ScenarioSingleClient::new();

    // Process a tick to get past the "on join" logic.
    app.update();
    helper.clear_received();

    // Insert a rain component to the instance.
    app.world.entity_mut(layer).insert(Rain(0.5));
    for _ in 0..2 {
        app.update();
    }

    // Alter a rain component of the instance.
    app.world.entity_mut(layer).insert(Rain(1.0));
    app.update();

    // Insert a thunder component to the instance.
    app.world.entity_mut(layer).insert(Thunder(0.5));
    app.update();

    // Alter a thunder component of the instance.
    app.world.entity_mut(layer).insert(Thunder(1.0));
    app.update();

    // Remove the rain component from the instance.
    app.world.entity_mut(layer).remove::<Rain>();
    for _ in 0..2 {
        app.update();
    }

    // Make assertions.
    let sent_packets = helper.collect_received();

    assert_weather_packets(sent_packets);
}

#[test]
fn test_weather_client() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer,
    } = ScenarioSingleClient::new();

    // Process a tick to get past the "on join" logic.
    app.update();
    helper.clear_received();

    // Insert a rain component to the client.
    app.world.entity_mut(client).insert(Rain(0.5));
    for _ in 0..2 {
        app.update();
    }

    // Alter a rain component of the client.
    app.world.entity_mut(client).insert(Rain(1.0));
    app.update();

    // Insert a thunder component to the client.
    app.world.entity_mut(client).insert(Thunder(0.5));
    app.update();

    // Alter a thunder component of the client.
    app.world.entity_mut(client).insert(Thunder(1.0));
    app.update();

    // Remove the rain component from the client.
    app.world.entity_mut(client).remove::<Rain>();
    for _ in 0..2 {
        app.update();
    }

    // Make assertions.
    let sent_packets = helper.collect_received();

    assert_weather_packets(sent_packets);
}

#[track_caller]
fn assert_weather_packets(sent_packets: PacketFrames) {
    sent_packets.assert_count::<GameStateChangeS2c>(6);
}
*/
