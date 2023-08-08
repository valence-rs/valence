use std::fs::{self, File};
use std::io::{self, BufReader, Write};
use std::time::Instant;

use anyhow::Context;
use flate2::bufread::GzEncoder;
use flate2::Compression;
use futures::StreamExt;
use indicatif::ProgressBar;
use valence_nbt::{compound, List};

/// Update me when a new MC version is released.
///
/// Get the new link from mcversions.net.
const DOWNLOAD_URL: &str =
    "https://piston-data.mojang.com/v1/objects/84194a2f286ef7c14ed7ce0090dba59902951553/server.jar";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let start_time = Instant::now();

    let resp = reqwest::get(DOWNLOAD_URL).await?;

    let total_len = resp
        .content_length()
        .context("missing content length from response")?;

    let pb = ProgressBar::new(total_len);

    let mut stream = resp.bytes_stream();

    fs::create_dir_all("vanilla-server")?;

    let mut jar_file = File::create("vanilla-server/server.jar")?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;

        pb.set_position((pb.position() + chunk.len() as u64).min(total_len));

        jar_file.write_all(&chunk)?;
    }

    pb.finish_and_clear();

    fs::write(
        "vanilla-server/server.properties",
        include_str!("../server.properties"),
    )?;

    fs::create_dir_all("vanilla-server/world")?;

    // Create minimal level.dat

    let data = compound! {
        "Data" => compound! {
            "RandomSeed" => 42_i64,
            "allowCommands" => true,
            "DayTime" => 6000, // Noon
            "GameRules" => compound! {
                "announceAdvancements" => "false",
                "doDaylightCycle" => "false",
                "doInsomnia" => "false",
                "doMobSpawning" => "false",
                "doWeatherCycle" => "false",
            },
            "GameType" => 1, // Creative
            "WorldGenSettings" => compound! {
                "bonus_chest" => false,
                "seed" => 12345_i64,
                "generate_features" => false,
                "dimensions" => compound! {
                    "minecraft:overworld" => compound! {
                        "generator" => compound! {
                            "type" => "flat",
                            "settings" => compound! {
                                // "biome" => "minecraft:plains",
                                // "features" => false,
                                // "lakes" => false,
                                "layers" => List::Compound(vec![
                                    compound! {
                                        "block" => "minecraft:bedrock",
                                        "height" => 1,
                                    },
                                    compound! {
                                        "block" => "minecraft:dirt",
                                        "height" => 2,
                                    },
                                    compound! {
                                        "block" => "minecraft:grass_block",
                                        "height" => 1,
                                    }
                                ])
                            },
                        },
                        "type" => "minecraft:overworld",
                    },
                    "minecraft:the_nether" => compound! {
                        "generator" => compound! {
                            "settings" => "minecraft:nether",
                            "biome_source" => compound! {
                                "preset" => "minecraft:nether",
                                "type" => "minecraft:multi_noise",
                            },
                            "type" => "minecraft:noise"
                        },
                        "type" => "minecraft:the_nether",
                    },
                    "minecraft:the_end" => compound! {
                        "generator" => compound! {
                            "settings" => "minecraft:end",
                            "biome_source" => compound! {
                                "type" => "minecraft:the_end",
                            },
                            "type" => "minecraft:noise",
                        },
                        "type" => "minecraft:the_end",
                    }
                }
            },
            "MapFeatures" => false,
            "initialized" => true,
        }
    };

    let mut buf = vec![];
    data.to_binary(&mut buf, "")?;
    let mut buf_slice = buf.as_slice();

    // Gzip the level.dat before writing to the file.
    let mut dat_file = File::create("vanilla-server/world/level.dat")?;
    let mut reader = BufReader::new(GzEncoder::new(&mut buf_slice, Compression::default()));
    io::copy(&mut reader, &mut dat_file)?;

    println!(
        "Done! Finished in {:.2} seconds.",
        Instant::now().duration_since(start_time).as_secs_f64()
    );

    Ok(())
}
