use std::path::{Path, PathBuf};
use std::str::FromStr;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tokio::runtime::Builder;
use valence::biome::BiomeId;
use valence::chunk::ChunkPos;
use valence::config::Config;
use valence_anvil::biome::BiomeKind;
use valence_anvil::AnvilWorld;
use std::process::{Command, Stdio};
use std::io::Write;
use std::fs::create_dir_all;

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);

const BENCHMARK_WORLD_ASSET: GitAsset = GitAsset::new(
    "https://github.com/TerminatorNL/valence-test-data.git",
    "Worlds/1.19.2/Benchmark world SP"
);

struct GitAsset<'a> {
    repository: &'a str,
    repo_path: &'a str,
}

impl<'a> GitAsset<'a> {
    pub const fn new(repository: &'a str, repo_path: &'a str) -> Self {
        GitAsset {
            repository,
            repo_path
        }
    }

    /// Downloads the asset from git if they aren't already downloaded.
    /// This download uses the 'git' command.
    /// Returns the location of the downloaded asset.
    pub fn load(&self) -> PathBuf {
        let asset_cache_dir = PathBuf::from_str(".asset_cache").unwrap();


        create_dir_all(&asset_cache_dir).expect("Unable to create `.asset_cache`");
        let asset_cache_dir = asset_cache_dir.canonicalize().expect("Unable to resolve `.asset_cache` directory");

//       let cmd = Command::new("git").current_dir(&asset_cache_dir).args(["clone", ""]).spawn().expect("Failed to execute `git clone` command");
//       std::io::stdout().write_all(&cmd.wait_with_output().expect("Failed to get `git clone` command output").stdout);
//
//       let cmd = Command::new("git").current_dir(&asset_cache_dir).args(["sparse-checkout", "set", self.repo_path]).spawn().expect("Failed to execute `git sparse-checkout` command");
//       std::io::stdout().write_all(&cmd.wait_with_output().expect("Failed to get `git sparse-checkout` command output").stdout);
//
//       let cmd = Command::new("pwd").current_dir(&asset_cache_dir).spawn().expect("Failed to execute `pwd` command");
//       std::io::stdout().write_all(&cmd.wait_with_output().expect("Failed to get `pwd` command output").stdout).expect("Unable to write output to console");

        unimplemented!("Asset downloading is not yet implemented")
    }
}

struct BenchmarkConfig;
impl Config for BenchmarkConfig {
    type ServerState = ();
    type ClientState = ();
    type EntityState = ();
    type WorldState = ();
    type ChunkState = ();
    type PlayerListState = ();
}

fn criterion_benchmark(c: &mut Criterion) {
    let world_directory = BENCHMARK_WORLD_ASSET.load();

    let world = AnvilWorld::new::<BenchmarkConfig, _>(
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
