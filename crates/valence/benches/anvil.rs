/*
use std::fs::create_dir_all;
use std::hint::black_box;
use std::path::{Path, PathBuf};

use anyhow::{ensure, Context};
use criterion::Criterion;
use fs_extra::dir::CopyOptions;
use reqwest::IntoUrl;
use valence::anvil::AnvilWorld;
use valence::instance::Chunk;
use zip::ZipArchive;

pub fn load(c: &mut Criterion) {
    let world_dir = get_world_asset(
        "https://github.com/valence-rs/valence-test-data/archive/refs/heads/asset/sp_world_1.19.2.zip",
        "1.19.2 benchmark world",
        true
    ).expect("failed to get world asset");

    let mut world = AnvilWorld::new(world_dir);

    c.bench_function("anvil_load_10x10", |b| {
        b.iter(|| {
            let world = black_box(&mut world);

            for z in -5..5 {
                for x in -5..5 {
                    let nbt = world
                        .read_chunk(x, z)
                        .expect("failed to read chunk")
                        .expect("missing chunk at position")
                        .data;

                    let mut chunk = Chunk::new(24);

                    valence_anvil::to_valence(&nbt, &mut chunk, 4, |_| Default::default()).unwrap();

                    black_box(chunk);
                }
            }
        });
    });
}

/// Loads the asset. If the asset is already present on the system due to a
/// prior run, the cached asset is used instead. If the asset is not
/// cached yet, this function downloads the asset using the current thread.
/// This will block until the download is complete.
///
/// returns: `PathBuf` The reference to the asset on the file system
fn get_world_asset(
    url: impl IntoUrl,
    dest_path: impl AsRef<Path>,
    remove_top_level_dir: bool,
) -> anyhow::Result<PathBuf> {
    let url = url.into_url()?;
    let dest_path = dest_path.as_ref();

    let asset_cache_dir = Path::new(".asset_cache");

    create_dir_all(asset_cache_dir).context("unable to create `.asset_cache` directory")?;

    let final_path = asset_cache_dir.join(dest_path);

    if final_path.exists() {
        return Ok(final_path);
    }

    let mut response = reqwest::blocking::get(url.clone())?;

    let cache_download_directory = asset_cache_dir.join("downloads");

    create_dir_all(&cache_download_directory)
        .context("unable to create `.asset_cache/downloads` directory")?;

    let mut downloaded_zip_file =
        tempfile::tempfile_in(&cache_download_directory).context("Could not create temp file")?;

    println!("Downloading {dest_path:?} from {url}");

    response
        .copy_to(&mut downloaded_zip_file)
        .context("could not write web contents to the temporary file")?;

    let mut zip_archive = ZipArchive::new(downloaded_zip_file)
        .context("unable to create zip archive from downloaded content")?;

    if !remove_top_level_dir {
        zip_archive
            .extract(&final_path)
            .context("unable to unzip downloaded contents")?;

        return Ok(final_path);
    }

    let temp_dir = tempfile::tempdir_in(&cache_download_directory)
        .context("unable to create temporary directory in `.asset_cache`")?;

    zip_archive
        .extract(&temp_dir)
        .context("unable to unzip downloaded contents")?;

    let mut entries = temp_dir.path().read_dir()?;

    let top_level_dir = entries
        .next()
        .context("the downloaded zip file was empty")??;

    ensure!(
        entries.next().is_none(),
        "found more than one entry in the top level directory of the Zip file"
    );

    ensure!(
        top_level_dir.path().is_dir(),
        "the only content in the zip archive is a file"
    );

    create_dir_all(&final_path).context("could not create a directory inside the asset cache")?;

    let dir_entries = top_level_dir
        .path()
        .read_dir()?
        .collect::<Result<Vec<_>, _>>()?;

    let items_to_move: Vec<_> = dir_entries.into_iter().map(|d| d.path()).collect();

    fs_extra::move_items(&items_to_move, &final_path, &CopyOptions::new())?;

    // We keep the temporary directory around until we're done moving files out
    // of it.
    drop(temp_dir);

    Ok(final_path)
}
*/
