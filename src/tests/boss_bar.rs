use valence_boss_bar::{
    BossBarBundle, BossBarColor, BossBarDivision, BossBarFlags, BossBarHealth, BossBarStyle,
    BossBarTitle,
};
use valence_client::VisibleEntityLayers;
use valence_core::despawn::Despawned;
use valence_core::text::{IntoText, Text};
use valence_entity::EntityLayerId;
use valence_packet::packets::play::BossBarS2c;

use crate::testing::ScenarioSingleClient;

#[test]
fn test_intialize_on_join() {
    let mut scenario = ScenarioSingleClient::new();

    // Insert a boss bar into the world
    scenario
        .app
        .world
        .entity_mut(scenario.layer)
        .insert(BossBarBundle {
            title: BossBarTitle("Boss Bar".into_text()),
            health: BossBarHealth(0.5),
            layer: EntityLayerId(scenario.layer),
            ..Default::default()
        });

    scenario.app.update();

    // We should receive a boss bar packet with the ADD action
    let frames = scenario.helper.collect_received();
    frames.assert_count::<BossBarS2c>(1);
}

#[test]
fn test_despawn() {
    let ScenarioSingleClient {
        mut app,
        mut helper,
        layer,
        ..
    } = prepare();

    // Despawn the boss bar
    app.world.entity_mut(layer).insert(Despawned);

    app.update();

    // We should receive a boss bar packet with the REMOVE action
    let frames = helper.collect_received();
    frames.assert_count::<BossBarS2c>(1);
}

#[test]
fn test_title_update() {
    let ScenarioSingleClient {
        mut app,
        mut helper,
        layer,
        ..
    } = prepare();

    // Update the title
    app.world
        .entity_mut(layer)
        .insert(BossBarTitle(Text::text("Test 2")));

    app.update();

    // We should receive a boss bar packet with the UPDATE_TITLE action
    let frames = helper.collect_received();
    frames.assert_count::<BossBarS2c>(1);
}

#[test]
fn test_health_update() {
    let ScenarioSingleClient {
        mut app,
        mut helper,
        layer,
        ..
    } = prepare();

    // Update the health
    app.world.entity_mut(layer).insert(BossBarHealth(0.5));

    app.update();

    // We should receive a boss bar packet with the UPDATE_HEALTH action
    let frames = helper.collect_received();
    frames.assert_count::<BossBarS2c>(1);
}

#[test]
fn test_style_update() {
    let ScenarioSingleClient {
        mut app,
        mut helper,
        layer,
        ..
    } = prepare();

    // Update the style
    app.world.entity_mut(layer).insert(BossBarStyle {
        color: BossBarColor::Red,
        division: BossBarDivision::TenNotches,
    });

    app.update();

    // We should receive a boss bar packet with the UPDATE_STYLE action
    let frames = helper.collect_received();
    frames.assert_count::<BossBarS2c>(1);
}

#[test]
fn test_flags_update() {
    let ScenarioSingleClient {
        mut app,
        mut helper,
        layer,
        ..
    } = prepare();

    // Update the flags
    let mut new_flags = BossBarFlags::new();
    new_flags.set_create_fog(true);
    app.world.entity_mut(layer).insert(new_flags);

    app.update();

    // We should receive a boss bar packet with the UPDATE_FLAGS action
    let frames = helper.collect_received();
    frames.assert_count::<BossBarS2c>(1);
}

#[test]
fn test_client_layer_change() {
    let ScenarioSingleClient {
        mut app,
        mut helper,
        layer,
        client,
    } = prepare();

    // Remove the layer from the client
    {
        let mut visible_entity_layers = app.world.get_mut::<VisibleEntityLayers>(client).unwrap();
        visible_entity_layers.0.clear();
    }

    app.update();

    // We should receive a boss bar packet with the REMOVE action
    let frames = helper.collect_received();
    frames.assert_count::<BossBarS2c>(1);

    // Add the layer back to the client
    {
        let mut visible_entity_layers = app.world.get_mut::<VisibleEntityLayers>(client).unwrap();
        visible_entity_layers.0.insert(layer);
    }

    app.update();

    // We should receive a boss bar packet with the ADD action
    let frames = helper.collect_received();
    frames.assert_count::<BossBarS2c>(1);
}

fn prepare() -> ScenarioSingleClient {
    let mut s = ScenarioSingleClient::new();

    // Process a tick to get past the "on join" logic.
    s.app.update();
    s.helper.clear_received();

    // Insert a boss bar into the world

    // Attach the new boss bar to the layer for convenience.
    s.app.world.entity_mut(s.layer).insert(BossBarBundle {
        title: BossBarTitle("Boss Bar".into_text()),
        health: BossBarHealth(0.5),
        layer: EntityLayerId(s.layer),
        ..Default::default()
    });

    for _ in 0..2 {
        s.app.update();
    }

    s.helper.clear_received();
    s
}
