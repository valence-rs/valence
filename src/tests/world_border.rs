use crate::protocol::packets::play::{
    WorldBorderCenterChangedS2c, WorldBorderInitializeS2c, WorldBorderInterpolateSizeS2c,
    WorldBorderSizeChangedS2c, WorldBorderWarningBlocksChangedS2c,
    WorldBorderWarningTimeChangedS2c,
};
use crate::testing::*;
use crate::world_border::{
    WorldBorderBundle, WorldBorderCenter, WorldBorderLerp, WorldBorderPortalTpBoundary,
    WorldBorderWarnBlocks, WorldBorderWarnTime,
};

#[test]
fn test_intialize_on_join() {
    let ScenarioSingleClient {
        mut app,
        client: _,
        mut helper,
        layer: _,
    } = prepare();

    app.update();

    // Check if a world border initialize packet was sent
    let frames = helper.collect_received();
    frames.assert_count::<WorldBorderInitializeS2c>(1);
}

#[test]
fn test_center_change() {
    let ScenarioSingleClient {
        mut app,
        client: _,
        mut helper,
        layer,
    } = prepare();

    app.update();

    helper.clear_received();

    // Change the center
    let mut center = app.world.get_mut::<WorldBorderCenter>(layer).unwrap();
    center.x = 10.0;

    app.update();

    // Check if a world border center changed packet was sent
    let frames = helper.collect_received();
    frames.assert_count::<WorldBorderCenterChangedS2c>(1);
}

#[test]
fn test_diameter_change() {
    let ScenarioSingleClient {
        mut app,
        client: _,
        mut helper,
        layer,
    } = prepare();

    app.update();

    helper.clear_received();

    // Change the diameter
    let mut lerp = app.world.get_mut::<WorldBorderLerp>(layer).unwrap();
    lerp.target_diameter = 20.0;

    app.update();

    // Check if a world border size changed packet was sent
    let frames = helper.collect_received();
    frames.assert_count::<WorldBorderSizeChangedS2c>(1);
}

#[test]
fn test_interpolation() {
    let ScenarioSingleClient {
        mut app,
        client: _,
        mut helper,
        layer,
    } = prepare();

    app.update();

    helper.clear_received();

    // Change the diameter and start interpolation to it over 20 ticks
    let mut lerp = app.world.get_mut::<WorldBorderLerp>(layer).unwrap();
    lerp.target_diameter = 20.0;
    lerp.remaining_ticks = 20;

    // Tick 20 times
    for _ in 0..20 {
        app.update();
    }

    // Check if a world border interpolate size packet was sent
    let frames = helper.collect_received();
    frames.assert_count::<WorldBorderInterpolateSizeS2c>(1);

    // Check if the interpolation is finished
    let lerp = app.world.get_mut::<WorldBorderLerp>(layer).unwrap();
    assert_eq!(lerp.current_diameter, 20.0);
    assert_eq!(lerp.remaining_ticks, 0);
}

#[test]
fn test_warning_blocks_change() {
    let ScenarioSingleClient {
        mut app,
        client: _,
        mut helper,
        layer,
    } = prepare();

    app.update();

    helper.clear_received();

    // Change the warning blocks
    let mut warn_blocks = app.world.get_mut::<WorldBorderWarnBlocks>(layer).unwrap();
    warn_blocks.0 = 10;

    app.update();

    // Check if a world border warning blocks changed packet was sent
    let frames = helper.collect_received();
    frames.assert_count::<WorldBorderWarningBlocksChangedS2c>(1);
}

#[test]
fn test_warning_time_change() {
    let ScenarioSingleClient {
        mut app,
        client: _,
        mut helper,
        layer,
    } = prepare();

    app.update();

    helper.clear_received();

    // Change the warning time
    let mut warn_time = app.world.get_mut::<WorldBorderWarnTime>(layer).unwrap();
    warn_time.0 = 10;

    app.update();

    // Check if a world border warning time changed packet was sent
    let frames = helper.collect_received();
    frames.assert_count::<WorldBorderWarningTimeChangedS2c>(1);
}

#[test]
fn test_portal_tp_boundary_change() {
    let ScenarioSingleClient {
        mut app,
        client: _,
        mut helper,
        layer,
    } = prepare();

    app.update();

    helper.clear_received();

    // Change the portal tp boundary
    let mut portal_tp_boundary = app
        .world
        .get_mut::<WorldBorderPortalTpBoundary>(layer)
        .unwrap();
    portal_tp_boundary.0 = 10;

    app.update();

    // Check if a world border initialize packet was sent
    let frames = helper.collect_received();
    frames.assert_count::<WorldBorderInitializeS2c>(1);
}

fn prepare() -> ScenarioSingleClient {
    let mut s = ScenarioSingleClient::new();

    // Process a tick to get past the "on join" logic.
    s.app.update();

    // Attach the world border bundle to the chunk layer
    s.app.world.entity_mut(s.layer).insert(WorldBorderBundle {
        lerp: WorldBorderLerp {
            target_diameter: 10.0,
            ..Default::default()
        },
        ..Default::default()
    });

    s
}
