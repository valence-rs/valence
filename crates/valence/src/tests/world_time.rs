use bevy_app::App;
use valence_entity::Location;
use valence_world_time::packet::WorldTimeUpdateS2c;
use valence_world_time::{
    ChangeTrackingTimeBroadcast, DayPhase, IntervalTimeBroadcast, LinearTimeTicking,
    LinearWorldAging, MoonPhase, WorldTime, DAY_LENGTH,
};

use super::scenario_single_client;

#[test]
fn test_world_time_add() {
    let mut time = WorldTime::default();
    time.add_time(10);

    assert_eq!(10, time.time_of_day);
    assert!(time.client_time_ticking());

    time.set_client_time_ticking(false);
    assert_eq!(-10, time.time_of_day);

    time.add_time(-11);
    assert_eq!(-i64::MAX, time.time_of_day);
}

#[test]
fn test_world_time_modifications() {
    let mut time = WorldTime::default();

    time.set_day(3);
    time.set_current_day_time(12000);
    assert_eq!(3 * DAY_LENGTH + 12000, time.time_of_day);

    time.warp_to_next_day_phase(DayPhase::Day);
    assert_eq!(4 * DAY_LENGTH, time.time_of_day);

    time.set_day(0);
    time.wrap_to_next_moon_phase(MoonPhase::NewMoon);
    assert_eq!(4 * DAY_LENGTH + DayPhase::Night as i64, time.time_of_day)
}

#[test]
fn test_interval_time_broadcast() {
    let mut app = App::new();
    let (client, mut client_helper) = scenario_single_client(&mut app);
    let loc: &Location = app.world.entity(client).get().unwrap();

    app.world
        .entity_mut(loc.0)
        .insert((WorldTime::default(), IntervalTimeBroadcast::new(20)));

    for _ in 0..20 {
        app.update()
    }

    client_helper
        .collect_sent()
        .assert_count::<WorldTimeUpdateS2c>(2);
}

#[test]
fn test_change_tracking_broadcast() {
    let mut app = App::new();
    let (client, mut client_helper) = scenario_single_client(&mut app);
    let loc: &Location = app.world.entity(client).get().unwrap();
    let ins_ent = loc.0;

    app.world
        .entity_mut(ins_ent)
        .insert((WorldTime::default(), ChangeTrackingTimeBroadcast));

    app.world
        .entity_mut(ins_ent)
        .get_mut::<WorldTime>()
        .unwrap()
        .add_time(1);

    app.update();
    client_helper
        .collect_sent()
        .assert_count::<WorldTimeUpdateS2c>(1);
}

#[test]
fn test_time_ticking() {
    let mut app = App::new();
    let (client, _) = scenario_single_client(&mut app);
    let loc: &Location = app.world.entity(client).get().unwrap();
    let ins_ent = loc.0;

    app.world.entity_mut(ins_ent).insert((
        WorldTime::default(),
        LinearTimeTicking { speed: 1 },
        LinearWorldAging { speed: 1 },
    ));

    app.update();

    let time: &WorldTime = app.world.entity(ins_ent).get().unwrap();
    assert_eq!(1, time.world_age);
    assert_eq!(1, time.time_of_day);
}
