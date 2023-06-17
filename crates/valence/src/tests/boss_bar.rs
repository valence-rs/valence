use bevy_app::App;
use bevy_ecs::entity::Entity;
use valence_boss_bar::{components::{BossBarBundle, BossBarViewers, BossBarColor, BossBarDivision, BossBarFlags, BossBarTitle, BossBarHealth, BossBarStyle}, packet::BossBarS2c};
use valence_core::{text::Text, despawn::Despawned};

use super::{MockClientHelper, scenario_single_client};

#[test]
fn test_intialize_on_join() {
    let mut app = App::new();
    let (client_ent, mut client_helper, instance_ent) = prepare(&mut app);

    let mut boss_bar = app.world.get_mut::<BossBarViewers>(instance_ent).unwrap();
    boss_bar.current_viewers.push(client_ent);

    app.update();

    let frames = client_helper.collect_sent();
    frames.assert_count::<BossBarS2c>(1);
}

#[test]
fn test_despawn() {
    let mut app = App::new();
    let (client_ent, mut client_helper, instance_ent) = prepare(&mut app);

    let mut boss_bar = app.world.get_mut::<BossBarViewers>(instance_ent).unwrap();
    boss_bar.current_viewers.push(client_ent);

    app.update();

    app.world.entity_mut(instance_ent).insert(Despawned);

    app.update();

    let frames = client_helper.collect_sent();
    frames.assert_count::<BossBarS2c>(2);
}

#[test]
fn test_title_update() {
    let mut app = App::new();
    let (client_ent, mut client_helper, instance_ent) = prepare(&mut app);

    let mut boss_bar = app.world.get_mut::<BossBarViewers>(instance_ent).unwrap();
    boss_bar.current_viewers.push(client_ent);

    app.update();

    app.world.entity_mut(instance_ent).insert(BossBarTitle(Text::text("Test 2")));

    app.update();

    let frames = client_helper.collect_sent();
    frames.assert_count::<BossBarS2c>(2);
}

#[test]
fn test_health_update() {
    let mut app = App::new();
    let (client_ent, mut client_helper, instance_ent) = prepare(&mut app);

    let mut boss_bar = app.world.get_mut::<BossBarViewers>(instance_ent).unwrap();
    boss_bar.current_viewers.push(client_ent);

    app.update();

    app.world.entity_mut(instance_ent).insert(BossBarHealth(0.5));

    app.update();

    let frames = client_helper.collect_sent();
    frames.assert_count::<BossBarS2c>(2);
}

#[test]
fn test_style_update() {
    let mut app = App::new();
    let (client_ent, mut client_helper, instance_ent) = prepare(&mut app);

    let mut boss_bar = app.world.get_mut::<BossBarViewers>(instance_ent).unwrap();
    boss_bar.current_viewers.push(client_ent);

    app.update();

    app.world.entity_mut(instance_ent).insert(BossBarStyle{
        color: BossBarColor::Red, division: BossBarDivision::TenNotches
    });

    app.update();

    let frames = client_helper.collect_sent();
    frames.assert_count::<BossBarS2c>(2);
}

#[test]
fn test_flags_update() {
    let mut app = App::new();
    let (client_ent, mut client_helper, instance_ent) = prepare(&mut app);

    let mut boss_bar = app.world.get_mut::<BossBarViewers>(instance_ent).unwrap();
    boss_bar.current_viewers.push(client_ent);

    app.update();

    let mut new_flags = BossBarFlags::new();
    new_flags.set_create_fog(true);
    app.world.entity_mut(instance_ent).insert(new_flags);

    app.update();

    let frames = client_helper.collect_sent();
    frames.assert_count::<BossBarS2c>(2);
}

fn prepare(app: &mut App) -> (Entity, MockClientHelper, Entity) {
    let (client_ent, mut client_helper) = scenario_single_client(app);

    // Process a tick to get past the "on join" logic.
    app.update();
    client_helper.clear_sent();

    // Insert a boss bar into the world
    let boss_bar = app.world.spawn(BossBarBundle::new(Text::text("Test"), BossBarColor::Blue, BossBarDivision::SixNotches, BossBarFlags::new())).id();
    
    for _ in 0..2 {
        app.update();
    }

    client_helper.clear_sent();
    (client_ent, client_helper, boss_bar)
}
