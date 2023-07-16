use std::time::Duration;

use bevy_app::prelude::*;
use criterion::Criterion;
use glam::DVec3;
use rand::Rng;
use valence::testing::create_mock_client;
use valence::DefaultPlugins;
use valence_biome::BiomeRegistry;
use valence_client::hand_swing::HandSwingC2s;
use valence_client::keepalive::KeepaliveSettings;
use valence_client::movement::FullC2s;
use valence_core::chunk_pos::ChunkPos;
use valence_core::{ident, CoreSettings, Server};
use valence_dimension::DimensionTypeRegistry;
use valence_entity::Position;
use valence_instance::chunk::UnloadedChunk;
use valence_instance::Instance;
use valence_layer::{ChunkLayer, EntityLayer};
use valence_network::NetworkPlugin;

pub fn many_players(c: &mut Criterion) {
    run_many_players(c, "many_players", 3000, 20, 16);
    run_many_players(c, "many_players_spread_out", 3000, 8, 200);
}

fn run_many_players(
    c: &mut Criterion,
    func_name: &str,
    client_count: usize,
    view_dist: u8,
    world_size: i32,
) {
    let mut app = App::new();

    app.insert_resource(CoreSettings {
        compression_threshold: Some(256),
        ..Default::default()
    });

    app.insert_resource(KeepaliveSettings {
        period: Duration::MAX,
    });

    app.add_plugins(DefaultPlugins.build().disable::<NetworkPlugin>());

    app.update(); // Initialize plugins.

    let mut chunks = ChunkLayer::new(
        ident!("overworld"),
        app.world.resource::<DimensionTypeRegistry>(),
        app.world.resource::<BiomeRegistry>(),
        app.world.resource::<Server>(),
    );

    for z in -world_size..world_size {
        for x in -world_size..world_size {
            chunks.insert_chunk(ChunkPos::new(x, z), UnloadedChunk::new());
        }
    }

    let layer = app
        .world
        .spawn((chunks, EntityLayer::new(app.world.resource::<Server>())))
        .id();

    let mut clients = vec![];

    // Spawn a bunch of clients in at random initial positions in the instance.
    for i in 0..client_count {
        let (mut bundle, helper) = create_mock_client(format!("client_{i}"));

        bundle.visible_chunk_layer.0 = layer;
        bundle.visible_entity_layers.0.insert(layer);
        bundle.player.layer.0 = layer;
        bundle.view_distance.set(view_dist);

        let mut rng = rand::thread_rng();
        let x = rng.gen_range(-world_size as f64 * 16.0..=world_size as f64 * 16.0);
        let z = rng.gen_range(-world_size as f64 * 16.0..=world_size as f64 * 16.0);

        bundle.player.position.set(DVec3::new(x, 64.0, z));

        let id = app.world.spawn(bundle).id();

        clients.push((id, helper));
    }

    let mut query = app.world.query::<&mut Position>();

    app.update();

    for (_, helper) in &mut clients {
        helper.confirm_initial_pending_teleports();
    }

    app.update();

    c.bench_function(func_name, |b| {
        b.iter(|| {
            let mut rng = rand::thread_rng();

            // Move the clients around randomly. They'll cross chunk borders and cause
            // interesting things to happen.
            for (id, helper) in &mut clients {
                let pos = query.get(&app.world, *id).unwrap().get();

                let offset = DVec3::new(rng.gen_range(-2.0..=2.0), 0.0, rng.gen_range(-2.0..=2.0));

                helper.send(&FullC2s {
                    position: pos + offset,
                    yaw: rng.gen_range(0.0..=360.0),
                    pitch: rng.gen_range(0.0..=360.0),
                    on_ground: rng.gen(),
                });

                helper.send(&HandSwingC2s {
                    hand: valence_core::hand::Hand::Main,
                });
            }

            drop(rng);

            app.update(); // The important part.

            for (_, helper) in &mut clients {
                helper.clear_received();
            }
        });
    });
}
