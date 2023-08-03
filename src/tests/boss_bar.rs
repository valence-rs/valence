use valence_boss_bar::{
    BossBarBundle, BossBarColor, BossBarDivision, BossBarFlags, BossBarHealth, BossBarStyle,
    BossBarTitle, BossBarViewers,
};
use valence_core::despawn::Despawned;
use valence_core::text::Text;
use valence_packet::packets::play::BossBarS2c;

use crate::testing::ScenarioSingleClient;

#[test]
fn test_intialize_on_join() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer,
    } = prepare();

    // Fetch the boss bar component
    let mut boss_bar = app.world.get_mut::<BossBarViewers>(layer).unwrap();
    // Add our mock client to the viewers list
    assert!(boss_bar.viewers.insert(client));

    app.update();

    // Check if a boss bar packet was sent
    let frames = helper.collect_received();
    frames.assert_count::<BossBarS2c>(1);
}

#[test]
fn test_despawn() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer,
    } = prepare();

    // Fetch the boss bar component
    let mut boss_bar = app.world.get_mut::<BossBarViewers>(layer).unwrap();
    // Add our mock client to the viewers list
    assert!(boss_bar.viewers.insert(client));

    app.update();

    // Despawn the boss bar
    app.world.entity_mut(layer).insert(Despawned);

    app.update();

    // Check if a boss bar packet was sent in addition to the ADD packet, which
    // should be a Remove packet
    let frames = helper.collect_received();
    frames.assert_count::<BossBarS2c>(2);
}

#[test]
fn test_title_update() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer,
    } = prepare();

    // Fetch the boss bar component
    let mut boss_bar = app.world.get_mut::<BossBarViewers>(layer).unwrap();
    // Add our mock client to the viewers list
    assert!(boss_bar.viewers.insert(client));

    app.update();

    // Update the title
    app.world
        .entity_mut(layer)
        .insert(BossBarTitle(Text::text("Test 2")));

    app.update();

    // Check if a boss bar packet was sent in addition to the ADD packet, which
    // should be an UpdateTitle packet
    let frames = helper.collect_received();
    frames.assert_count::<BossBarS2c>(2);
}

#[test]
fn test_health_update() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer,
    } = prepare();

    // Fetch the boss bar component
    let mut boss_bar = app.world.get_mut::<BossBarViewers>(layer).unwrap();
    // Add our mock client to the viewers list
    assert!(boss_bar.viewers.insert(client));

    app.update();

    // Update the health
    app.world.entity_mut(layer).insert(BossBarHealth(0.5));

    app.update();

    // Check if a boss bar packet was sent in addition to the ADD packet, which
    // should be an UpdateHealth packet
    let frames = helper.collect_received();
    frames.assert_count::<BossBarS2c>(2);
}

#[test]
fn test_style_update() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer,
    } = prepare();

    // Fetch the boss bar component
    let mut boss_bar = app.world.get_mut::<BossBarViewers>(layer).unwrap();
    // Add our mock client to the viewers list
    assert!(boss_bar.viewers.insert(client));

    app.update();

    // Update the style
    app.world.entity_mut(layer).insert(BossBarStyle {
        color: BossBarColor::Red,
        division: BossBarDivision::TenNotches,
    });

    app.update();

    // Check if a boss bar packet was sent in addition to the ADD packet, which
    // should be an UpdateStyle packet
    let frames = helper.collect_received();
    frames.assert_count::<BossBarS2c>(2);
}

#[test]
fn test_flags_update() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer,
    } = prepare();

    // Fetch the boss bar component
    let mut boss_bar = app.world.get_mut::<BossBarViewers>(layer).unwrap();
    // Add our mock client to the viewers list
    assert!(boss_bar.viewers.insert(client));

    app.update();

    // Update the flags
    let mut new_flags = BossBarFlags::new();
    new_flags.set_create_fog(true);
    app.world.entity_mut(layer).insert(new_flags);

    app.update();

    // Check if a boss bar packet was sent in addition to the ADD packet, which
    // should be an UpdateFlags packet
    let frames = helper.collect_received();
    frames.assert_count::<BossBarS2c>(2);
}

#[test]
fn test_client_disconnection() {
    let ScenarioSingleClient {
        mut app,
        client,
        mut helper,
        layer,
    } = prepare();

    // Fetch the boss bar component
    let mut boss_bar = app.world.get_mut::<BossBarViewers>(layer).unwrap();
    // Add our mock client to the viewers list
    assert!(boss_bar.viewers.insert(client));

    app.update();

    // Remove the client from the world
    app.world.entity_mut(client).insert(Despawned);

    app.update();

    assert!(app
        .world
        .get_mut::<BossBarViewers>(layer)
        .unwrap()
        .viewers
        .is_empty());

    // Check if a boss bar packet was sent in addition to the ADD packet, which
    // should be a Remove packet
    let frames = helper.collect_received();
    frames.assert_count::<BossBarS2c>(2);
}

fn prepare() -> ScenarioSingleClient {
    let mut s = ScenarioSingleClient::new();

    // Process a tick to get past the "on join" logic.
    s.app.update();
    s.helper.clear_received();

    // Insert a boss bar into the world

    // Attach the new boss bar to the layer for convenience.
    s.app.world.entity_mut(s.layer).insert(BossBarBundle {
        title: BossBarTitle(Text::text("Test")),
        style: BossBarStyle {
            color: BossBarColor::Blue,
            division: BossBarDivision::SixNotches,
        },
        flags: BossBarFlags::new(),
        ..Default::default()
    });

    for _ in 0..2 {
        s.app.update();
    }

    s.helper.clear_received();
    s
}
