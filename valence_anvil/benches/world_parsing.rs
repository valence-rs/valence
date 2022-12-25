use criterion::{black_box, criterion_group, criterion_main, Criterion};
use valence::chunk::{ChunkPos, UnloadedChunk};
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

fn criterion_benchmark(c: &mut Criterion) {
    let world_dir = BENCHMARK_WORLD_ASSET.load_blocking_panic();

    let mut world = AnvilWorld::new(world_dir);

    let mut load_targets = Vec::new();
    for x in -5..5 {
        for z in -5..5 {
            load_targets.push(ChunkPos::new(x, z));
        }
    }

    c.bench_function("Load square 10x10", |b| {
        b.iter(|| {
            let world = black_box(&mut world);

            for z in -5..5 {
                for x in -5..5 {
                    let nbt = world
                        .read_chunk(x, z)
                        .expect("failed to read chunk")
                        .expect("missing chunk at position")
                        .data;

                    let mut chunk = UnloadedChunk::new(24);

                    valence_anvil::to_valence(&nbt, &mut chunk, 4, |_| Default::default()).unwrap();

                    black_box(chunk);
                }
            }
        });
    });
}
