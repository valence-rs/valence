use bevy_app::App;
use valence_client::weather::{Rain, Thunder};
use valence_client::Client;
use valence_core::packet::s2c::play::game_state_change::GameEventKind;
use valence_core::packet::s2c::play::{GameStateChangeS2c, S2cPlayPacket};

use super::*;

fn assert_weather_packets(sent_packets: Vec<S2cPlayPacket>) {
    assert_packet_count!(sent_packets, 6, S2cPlayPacket::GameStateChangeS2c(_));

    assert_packet_order!(
        sent_packets,
        S2cPlayPacket::GameStateChangeS2c(GameStateChangeS2c {
            kind: GameEventKind::BeginRaining,
            value: _
        }),
        S2cPlayPacket::GameStateChangeS2c(GameStateChangeS2c {
            kind: GameEventKind::RainLevelChange,
            value: _
        }),
        S2cPlayPacket::GameStateChangeS2c(GameStateChangeS2c {
            kind: GameEventKind::ThunderLevelChange,
            value: _
        }),
        S2cPlayPacket::GameStateChangeS2c(GameStateChangeS2c {
            kind: GameEventKind::EndRaining,
            value: _
        })
    );

    if let S2cPlayPacket::GameStateChangeS2c(pkt) = sent_packets[1] {
        assert_eq!(pkt.value, 0.5);
    }

    if let S2cPlayPacket::GameStateChangeS2c(pkt) = sent_packets[2] {
        assert_eq!(pkt.value, 1.0);
    }

    if let S2cPlayPacket::GameStateChangeS2c(pkt) = sent_packets[3] {
        assert_eq!(pkt.value, 0.5);
    }

    if let S2cPlayPacket::GameStateChangeS2c(pkt) = sent_packets[4] {
        assert_eq!(pkt.value, 1.0);
    }
}

#[test]
fn test_weather_instance() {
    let mut app = App::new();
    let (_, mut client_helper) = scenario_single_client(&mut app);

    // Process a tick to get past the "on join" logic.
    app.update();
    client_helper.clear_sent();

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
    let sent_packets = client_helper.collect_sent();

    assert_weather_packets(sent_packets);
}

#[test]
fn test_weather_client() {
    let mut app = App::new();
    let (_, mut client_helper) = scenario_single_client(&mut app);

    // Process a tick to get past the "on join" logic.
    app.update();
    client_helper.clear_sent();

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
    let sent_packets = client_helper.collect_sent();

    assert_weather_packets(sent_packets);
}
