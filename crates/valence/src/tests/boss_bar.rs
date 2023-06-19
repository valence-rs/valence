use bevy_app::App;
use bevy_ecs::entity::Entity;
use valence_boss_bar::packet::BossBarS2c;
use valence_boss_bar::{
    BossBarBundle, BossBarColor, BossBarDivision, BossBarFlags, BossBarHealth, BossBarStyle,
    BossBarTitle, BossBarViewers,
};
use valence_core::despawn::Despawned;
use valence_core::text::Text;

use super::{scenario_single_client, MockClientHelper};

#[test]
fn test_intialize_on_join() {
    let mut app = App::new();
    let (client_ent, mut client_helper, instance_ent) = prepare(&mut app);

    // Fetch the boss bar component
    let mut boss_bar = app.world.get_mut::<BossBarViewers>(instance_ent).unwrap();
    // Add our mock client to the viewers list
    assert!(boss_bar.viewers.insert(client_ent));

    app.update();

    // Check if a boss bar packet was sent
    let frames = client_helper.collect_sent();
    frames.assert_count::<BossBarS2c>(1);
}

#[test]
fn test_despawn() {
    let mut app = App::new();
    let (client_ent, mut client_helper, instance_ent) = prepare(&mut app);

    // Fetch the boss bar component
    let mut boss_bar = app.world.get_mut::<BossBarViewers>(instance_ent).unwrap();
    // Add our mock client to the viewers list
    assert!(boss_bar.viewers.insert(client_ent));

    app.update();

    // Despawn the boss bar
    app.world.entity_mut(instance_ent).insert(Despawned);

    app.update();

    // Check if a boss bar packet was sent in addition to the ADD packet, which
    // should be a Remove packet
    let frames = client_helper.collect_sent();
    frames.assert_count::<BossBarS2c>(2);
}

#[test]
fn test_title_update() {
    let mut app = App::new();
    let (client_ent, mut client_helper, instance_ent) = prepare(&mut app);

    // Fetch the boss bar component
    let mut boss_bar = app.world.get_mut::<BossBarViewers>(instance_ent).unwrap();
    // Add our mock client to the viewers list
    assert!(boss_bar.viewers.insert(client_ent));

    app.update();

    // Update the title
    app.world
        .entity_mut(instance_ent)
        .insert(BossBarTitle(Text::text("Test 2")));

    app.update();

    // Check if a boss bar packet was sent in addition to the ADD packet, which
    // should be an UpdateTitle packet
    let frames = client_helper.collect_sent();
    frames.assert_count::<BossBarS2c>(2);
}

#[test]
fn test_health_update() {
    let mut app = App::new();
    let (client_ent, mut client_helper, instance_ent) = prepare(&mut app);

    // Fetch the boss bar component
    let mut boss_bar = app.world.get_mut::<BossBarViewers>(instance_ent).unwrap();
    // Add our mock client to the viewers list
    assert!(boss_bar.viewers.insert(client_ent));

    app.update();

    // Update the health
    app.world
        .entity_mut(instance_ent)
        .insert(BossBarHealth(0.5));

    app.update();

    // Check if a boss bar packet was sent in addition to the ADD packet, which
    // should be an UpdateHealth packet
    let frames = client_helper.collect_sent();
    frames.assert_count::<BossBarS2c>(2);
}

#[test]
fn test_style_update() {
    let mut app = App::new();
    let (client_ent, mut client_helper, instance_ent) = prepare(&mut app);

    // Fetch the boss bar component
    let mut boss_bar = app.world.get_mut::<BossBarViewers>(instance_ent).unwrap();
    // Add our mock client to the viewers list
    assert!(boss_bar.viewers.insert(client_ent));

    app.update();

    // Update the style
    app.world.entity_mut(instance_ent).insert(BossBarStyle {
        color: BossBarColor::Red,
        division: BossBarDivision::TenNotches,
    });

    app.update();

    // Check if a boss bar packet was sent in addition to the ADD packet, which
    // should be an UpdateStyle packet
    let frames = client_helper.collect_sent();
    frames.assert_count::<BossBarS2c>(2);
}

#[test]
fn test_flags_update() {
    let mut app = App::new();
    let (client_ent, mut client_helper, instance_ent) = prepare(&mut app);

    // Fetch the boss bar component
    let mut boss_bar = app.world.get_mut::<BossBarViewers>(instance_ent).unwrap();
    // Add our mock client to the viewers list
    assert!(boss_bar.viewers.insert(client_ent));

    app.update();

    // Update the flags
    let mut new_flags = BossBarFlags::new();
    new_flags.set_create_fog(true);
    app.world.entity_mut(instance_ent).insert(new_flags);

    app.update();

    // Check if a boss bar packet was sent in addition to the ADD packet, which
    // should be an UpdateFlags packet
    let frames = client_helper.collect_sent();
    frames.assert_count::<BossBarS2c>(2);
}

#[test]
fn test_client_disconnection() {
    let mut app = App::new();
    let (client_ent, mut client_helper, instance_ent) = prepare(&mut app);

    // Fetch the boss bar component
    let mut boss_bar = app.world.get_mut::<BossBarViewers>(instance_ent).unwrap();
    // Add our mock client to the viewers list
    assert!(boss_bar.viewers.insert(client_ent));

    app.update();

    // Remove the client from the world
    app.world.entity_mut(client_ent).insert(Despawned);

    app.update();

    assert!(app
        .world
        .get_mut::<BossBarViewers>(instance_ent)
        .unwrap()
        .viewers
        .is_empty());

    // Check if a boss bar packet was sent in addition to the ADD packet, which
    // should be a Remove packet
    let frames = client_helper.collect_sent();
    frames.assert_count::<BossBarS2c>(2);
}

fn prepare(app: &mut App) -> (Entity, MockClientHelper, Entity) {
    let (client_ent, mut client_helper) = scenario_single_client(app);

    // Process a tick to get past the "on join" logic.
    app.update();
    client_helper.clear_sent();

    // Insert a boss bar into the world
    let boss_bar = app
        .world
        .spawn(BossBarBundle::new(
            Text::text("Test"),
            BossBarColor::Blue,
            BossBarDivision::SixNotches,
            BossBarFlags::new(),
        ))
        .id();

    for _ in 0..2 {
        app.update();
    }

    client_helper.clear_sent();
    (client_ent, client_helper, boss_bar)
}
