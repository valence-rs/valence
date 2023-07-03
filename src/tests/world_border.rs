use std::time::Duration;

use bevy_app::App;
use valence_entity::Location;
use valence_instance::Instance;
use valence_registry::{Entity, Mut};
use valence_world_border::packet::*;
use valence_world_border::*;

use crate::testing::{create_mock_client, MockClientHelper, scenario_single_client};

#[test]
fn test_intialize_on_join() {
    let mut app = App::new();
    let (_, instance_ent) = prepare(&mut app);

    let (client, mut client_helper) = create_mock_client("test");
    let client_ent = app.world.spawn(client).id();

    app.world.get_mut::<Location>(client_ent).unwrap().0 = instance_ent;
    app.update();

    client_helper
        .collect_received()
        .assert_count::<WorldBorderInitializeS2c>(1);
}

#[test]
fn test_resizing() {
    let mut app = App::new();
    let (mut client_helper, instance_ent) = prepare(&mut app);

    app.world.send_event(SetWorldBorderSizeEvent {
        new_diameter: 20.0,
        duration: Duration::ZERO,
        instance: instance_ent,
    });

    app.update();
    let frames = client_helper.collect_received();
    frames.assert_count::<WorldBorderSizeChangedS2c>(1);
}

#[test]
fn test_center() {
    let mut app = App::new();
    let (mut client_helper, instance_ent) = prepare(&mut app);

    let mut ins_mut = app.world.entity_mut(instance_ent);
    let mut center: Mut<WorldBorderCenter> = ins_mut
        .get_mut()
        .expect("Expect world border to be present!");
    center.0 = [10.0, 10.0].into();

    app.update();
    let frames = client_helper.collect_received();
    frames.assert_count::<WorldBorderCenterChangedS2c>(1);
}

#[test]
fn test_warn_time() {
    let mut app = App::new();
    let (mut client_helper, instance_ent) = prepare(&mut app);

    let mut ins_mut = app.world.entity_mut(instance_ent);
    let mut wt: Mut<WorldBorderWarnTime> = ins_mut
        .get_mut()
        .expect("Expect world border to be present!");
    wt.0 = 100;
    app.update();

    let frames = client_helper.collect_received();
    frames.assert_count::<WorldBorderWarningTimeChangedS2c>(1);
}

#[test]
fn test_warn_blocks() {
    let mut app = App::new();
    let (mut client_helper, instance_ent) = prepare(&mut app);

    let mut ins_mut = app.world.entity_mut(instance_ent);
    let mut wb: Mut<WorldBorderWarnBlocks> = ins_mut
        .get_mut()
        .expect("Expect world border to be present!");
    wb.0 = 100;
    app.update();

    let frames = client_helper.collect_received();
    frames.assert_count::<WorldBorderWarningBlocksChangedS2c>(1);
}

#[test]
fn test_portal_tp_boundary() {
    let mut app = App::new();
    let (mut client_helper, instance_ent) = prepare(&mut app);

    let mut ins_mut = app.world.entity_mut(instance_ent);
    let mut tp: Mut<WorldBorderPortalTpBoundary> = ins_mut
        .get_mut()
        .expect("Expect world border to be present!");
    tp.0 = 100;
    app.update();

    let frames = client_helper.collect_received();
    frames.assert_count::<WorldBorderInitializeS2c>(1);
}

fn prepare(app: &mut App) -> (MockClientHelper, Entity) {
    let (_, mut client_helper) = scenario_single_client(app);

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

    // Insert a the world border bundle to the instance.
    app.world
        .entity_mut(instance_ent)
        .insert(WorldBorderBundle::new([0.0, 0.0], 10.0));
    for _ in 0..2 {
        app.update();
    }

    client_helper.clear_received();
    (client_helper, instance_ent)
}
