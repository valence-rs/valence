use criterion::Criterion;
use valence::prelude::*;

/// Benches the performance of a single server tick while nothing much is
/// happening.
pub fn idle_update(c: &mut Criterion) {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins);
    app.add_systems(Startup, setup);

    // Run startup schedule.
    app.update();

    c.bench_function("idle_update", |b| {
        b.iter(|| {
            app.update();
        });
    });
}

fn setup(
    mut commands: Commands,
    dimensions: Res<DimensionTypeRegistry>,
    biomes: Res<BiomeRegistry>,
    server: Res<Server>,
) {
    let mut layer = CombinedLayerBundle::new(Default::default(), &dimensions, &biomes, &server);

    for z in -5..5 {
        for x in -5..5 {
            layer.chunk_index.insert([x, z], Chunk::new());
        }
    }

    for z in -50..50 {
        for x in -50..50 {
            layer
                .chunk_index
                .set_block([x, 64, z], BlockState::GRASS_BLOCK);
        }
    }

    commands.spawn(layer);
}
