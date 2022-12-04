use valence::biome::BiomeId;
use valence::chunk::ChunkPos;
use valence::config::Config;
use valence_anvil::biome::BiomeKind;
use valence_anvil::AnvilWorld;
use tokio::runtime::Builder;

#[path="../tests/assets.rs"]
pub mod assets;

const BENCHMARK_WORLD_ASSET: assets::WebAsset<&'static str, &'static str> = assets::WebAsset::zipped_directory(
    "1.19.2 benchmark world",
    true,
    "https://github.com/valence-rs/valence-test-data/archive/refs/heads/asset/sp_world_1.19.2.zip",
);

struct TestConfig;
impl Config for TestConfig {
    type ServerState = ();
    type ClientState = ();
    type EntityState = ();
    type WorldState = ();
    type ChunkState = ();
    type PlayerListState = ();
    type InventoryState = ();
}

#[test]
pub fn parse_world(){
    let world_directory = BENCHMARK_WORLD_ASSET.load_blocking_panic();
    let world = AnvilWorld::new::<TestConfig, _>(
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

    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Creating runtime failed");

    for (chunk_pos, chunk) in runtime.block_on(world.load_chunks(load_targets.into_iter())).unwrap() {
        assert!(
            chunk.is_some(),
            "Chunk at {chunk_pos:?} returned 'None'. Is this section of the world \
         generated?"
        );
    }
}