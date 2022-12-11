use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tokio::runtime::Builder;
use valence::biome::BiomeId;
use valence::chunk::ChunkPos;
use valence::config::Config;
use valence::dimension::Dimension;
use valence_anvil::biome::BiomeKind;
use valence_anvil::AnvilWorld;

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

#[path = "../tests/assets.rs"]
pub mod assets;

const BENCHMARK_WORLD_ASSET: assets::WebAsset<&'static str, &'static str> = assets::WebAsset::zipped_directory(
    "1.19.2 benchmark world",
    true,
    "https://github.com/valence-rs/valence-test-data/archive/refs/heads/asset/sp_world_1.19.2.zip",
);

struct BenchmarkConfig;
impl Config for BenchmarkConfig {
    type ServerState = ();
    type ClientState = ();
    type EntityState = ();
    type WorldState = ();
    type ChunkState = ();
    type PlayerListState = ();
    type InventoryState = ();
}

fn criterion_benchmark(c: &mut Criterion) {
    let world_directory = BENCHMARK_WORLD_ASSET.load_blocking_panic();

    let world = AnvilWorld::new::<BenchmarkConfig, _>(
        &Dimension::default(),
        world_directory,
        BiomeKind::ALL
            .iter()
            .map(|b| (BiomeId::default(), b.biome().unwrap())),
    );

    let mut load_targets = Vec::new();
    for x in -5..5 {
        for z in -5..5 {
            load_targets.push(ChunkPos::new(x, z));
        }
    }

    let runtime = Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Creating runtime failed");

    c.bench_function("Load square 10x10", |b| {
        b.to_async(&runtime).iter_with_setup(
            || load_targets.clone().into_iter(),
            |targets| async {
                for (chunk_pos, chunk) in world.load_chunks(black_box(targets)).await.unwrap() {
                    assert!(
                        chunk.is_some(),
                        "Chunk at {chunk_pos:?} returned 'None'. Is this section of the world \
                         generated?"
                    );
                    black_box(chunk);
                }
            },
        );
    });
}
