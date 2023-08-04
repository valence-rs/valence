use glam::DVec3;
use valence_client::abilities::PlayerAbilitiesFlags;
use valence_core::chunk_pos::ChunkPos;
use valence_core::game_mode::GameMode;
use valence_layer::chunk::UnloadedChunk;
use valence_layer::ChunkLayer;
use valence_packet::packets::play::{
    FullC2s, MoveRelativeS2c, PlayerPositionLookS2c, TeleportConfirmC2s, UpdatePlayerAbilitiesC2s,
};

use crate::testing::{create_mock_client, ScenarioSingleClient};

#[test]
fn client_teleport_and_move() {
    let ScenarioSingleClient {
        mut app,
        client: _,
        helper: mut helper_1,
        layer: layer_ent,
    } = ScenarioSingleClient::new();

    let mut layer = app.world.get_mut::<ChunkLayer>(layer_ent).unwrap();

    for z in -10..10 {
        for x in -10..10 {
            layer.insert_chunk(ChunkPos::new(x, z), UnloadedChunk::new());
        }
    }

    let (mut bundle, mut helper_2) = create_mock_client("other");

    bundle.player.layer.0 = layer_ent;
    bundle.visible_chunk_layer.0 = layer_ent;
    bundle.visible_entity_layers.0.insert(layer_ent);

    app.world.spawn(bundle);

    app.update();

    // Client received an initial teleport.
    helper_1
        .collect_received()
        .assert_count::<PlayerPositionLookS2c>(1);

    // Confirm the initial teleport from the server.
    helper_1.send(&TeleportConfirmC2s {
        teleport_id: 0.into(),
    });

    // Move a little.
    helper_1.send(&FullC2s {
        position: DVec3::new(1.0, 0.0, 0.0),
        yaw: 0.0,
        pitch: 0.0,
        on_ground: true,
    });

    app.update();

    // Check that the other client saw the client moving.
    helper_2
        .collect_received()
        .assert_count::<MoveRelativeS2c>(1);
}

#[test]
fn client_start_flying() {
    let mut senario = ScenarioSingleClient::new();

    assert!(!senario
        .app
        .world
        .get::<PlayerAbilitiesFlags>(senario.client)
        .unwrap()
        .flying());

    senario
        .helper
        .send::<UpdatePlayerAbilitiesC2s>(&UpdatePlayerAbilitiesC2s::StartFlying);

    senario.app.update();

    assert!(senario
        .app
        .world
        .get::<PlayerAbilitiesFlags>(senario.client)
        .unwrap()
        .flying());

    senario
        .helper
        .send::<UpdatePlayerAbilitiesC2s>(&UpdatePlayerAbilitiesC2s::StopFlying);

    senario.app.update();

    assert!(!senario
        .app
        .world
        .get::<PlayerAbilitiesFlags>(senario.client)
        .unwrap()
        .flying());
}

#[test]
fn client_gamemode_changed_ability() {
    let mut senario = ScenarioSingleClient::new();

    *senario
        .app
        .world
        .get_mut::<GameMode>(senario.client)
        .unwrap() = GameMode::Creative;

    senario.app.update();

    let abilities = senario
        .app
        .world
        .get::<PlayerAbilitiesFlags>(senario.client)
        .unwrap();

    assert!(abilities.allow_flying());
    assert!(abilities.instant_break());
    assert!(abilities.invulnerable());

    *senario
        .app
        .world
        .get_mut::<GameMode>(senario.client)
        .unwrap() = GameMode::Adventure;

    senario.app.update();

    let abilities = senario
        .app
        .world
        .get::<PlayerAbilitiesFlags>(senario.client)
        .unwrap();

    assert!(!abilities.allow_flying());
    assert!(!abilities.instant_break());
    assert!(!abilities.invulnerable());
}
