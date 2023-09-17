use valence_server::protocol::packets::play::WorldTimeUpdateS2c;
use valence_world_time::extra::{DayPhase, MoonPhase, DAY_LENGTH};
use valence_world_time::{WorldTime, WorldTimeBundle};

use crate::testing::ScenarioSingleClient;

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
    time.set_current_day_time(12000u64);
    assert_eq!(3 * DAY_LENGTH + 12000, time.time_of_day as u64);

    time.warp_to_next_day_phase(DayPhase::Day);
    assert_eq!(4 * DAY_LENGTH, time.time_of_day as u64);

    time.set_day(0);
    time.wrap_to_next_moon_phase(MoonPhase::NewMoon);
    assert_eq!(
        4 * DAY_LENGTH + DayPhase::Night as u64,
        time.time_of_day as u64
    )
}

#[test]
fn test_time_ticking_broadcast() {
    let ScenarioSingleClient {
        mut app,
        client: _,
        mut helper,
        layer,
    } = prepare();

    for _ in 0..40 {
        app.update()
    }

    helper
        .collect_received()
        .assert_count::<WorldTimeUpdateS2c>(2);

    let x: &WorldTime = app.world.get(layer).unwrap();
    assert_eq!(x.time_of_day, 40);
}

fn prepare() -> ScenarioSingleClient {
    let mut s = ScenarioSingleClient::new();

    // Process a tick to get past the "on join" logic.
    s.app.update();
    s.app
        .world
        .entity_mut(s.layer)
        .insert(WorldTimeBundle::default());
    s
}
