use std::fs::{create_dir_all, DirEntry};
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use fs_extra::dir::CopyOptions;
use reqwest::IntoUrl;

/// Describes where to find the asset if it already has been downloaded and from
/// which URL the asset can be downloaded. More asset types can be added on
/// demand by modifying this enum.
pub enum WebAsset<DestinationPath: AsRef<Path>, URL: IntoUrl> {
    ZippedDirectory {
        destination_path: DestinationPath,
        remove_top_level_dir: bool,
        url: URL,
    },
}

impl<DestinationPath: AsRef<Path>, URL: IntoUrl + Clone> WebAsset<DestinationPath, URL> {
    /// Creates a ZippedDirectory asset type.
    ///
    /// # Arguments
    ///
    /// * `destination_path`: A unique path for this asset. If the path is
    ///   relative, it will be placed under the `.asset_cache` directory.
    ///   Relative paths are recommended.
    /// * `remove_top_level_dir`: Some zip files wrap all their contents in an
    ///   additional folder. Setting this value to `true` will remove that
    ///   redundant directory. If the Zip file contains multiple
    ///   files/directories in the root, this will cause a panic.
    /// * `url`: The URL from which to download the Zip file.
    ///
    /// returns: `WebAsset<DestinationPath, URL>` The created asset.
    ///
    /// # Examples
    ///
    /// ```
    /// const BENCHMARK_WORLD_ASSET: benchtools::WebAsset<&'static str, &'static str> = benchtools::WebAsset::zipped_directory(
    ///     "BenchmarkWorld",
    ///     true,
    ///     "https://github.com/valence-rs/valence-test-data/archive/refs/heads/asset/sp_world_1.19.2.zip",
    /// );
    /// ```
    pub const fn zipped_directory(
        destination_path: DestinationPath,
        remove_top_level_dir: bool,
        url: URL,
    ) -> Self {
        Self::ZippedDirectory {
            destination_path,
            remove_top_level_dir,
            url,
        }
    }

    fn url(&self) -> URL {
        match self {
            WebAsset::ZippedDirectory { url, .. } => url.clone(),
        }
    }

    fn destination_path(&self) -> &DestinationPath {
        match self {
            WebAsset::ZippedDirectory {
                destination_path: directory_name,
                ..
            } => directory_name,
        }
    }

    /// Loads the asset. If the asset is already present on the system due to a
    /// prior run, the cached asset is used instead. If the asset is not
    /// cached yet, this function downloads the asset using the current thread.
    /// This will block until the download is complete.
    ///
    /// returns: `PathBuf` The reference to the asset on the file system
    ///
    /// # Examples
    ///
    /// ```
    /// const BENCHMARK_WORLD_ASSET: benchtools::WebAsset<&'static str, &'static str> = benchtools::WebAsset::zipped_directory(
    ///     "BenchmarkWorld",
    ///     true,
    ///     "https://github.com/valence-rs/valence-test-data/archive/refs/heads/asset/sp_world_1.19.2.zip",
    /// );
    /// let world_directory = BENCHMARK_WORLD_ASSET.load_blocking_panic();
    /// ```
    pub fn load_blocking_panic(&self) -> PathBuf {
        let asset_cache_dir = PathBuf::from_str(".asset_cache").unwrap();
        create_dir_all(&asset_cache_dir).expect("Unable to create `.asset_cache` directory");
        let final_path = asset_cache_dir.join(self.destination_path());
        if final_path.exists() {
            return final_path;
        }

        let mut request = reqwest::blocking::get(self.url())
            .expect("File download request failed")
            .error_for_status()
            .unwrap();

        let cache_download_directory = asset_cache_dir.join("downloads");
        create_dir_all(&cache_download_directory)
            .expect("Unable to create `.asset_cache/downloads` directory");

        let mut downloaded_zip_file = tempfile::tempfile_in(&cache_download_directory)
            .expect("Could not create the temporary file");

        println!(
            "Downloading {:?} from {}",
            self.destination_path().as_ref(),
            self.url().as_str()
        );
        request
            .copy_to(&mut downloaded_zip_file)
            .expect("Could not write web contents to the temporary file");

        match self {
            WebAsset::ZippedDirectory {
                remove_top_level_dir: remove_single_top_level_dir,
                ..
            } => {
                let mut zip_archive = zip::ZipArchive::new(downloaded_zip_file)
                    .expect("unable to create zip archive from downloaded content");
                if *remove_single_top_level_dir {
                    let temporary_directory = tempfile::tempdir_in(&cache_download_directory)
                        .expect("Unable to create temporary directory in `.asset_cache`");
                    zip_archive
                        .extract(&temporary_directory)
                        .expect("Unable to unzip downloaded contents");
                    let mut entries: Vec<io::Result<DirEntry>> = temporary_directory
                        .path()
                        .read_dir()
                        .expect("Could not read the contents of the temporary directory")
                        .into_iter()
                        .collect();
                    if let Some(top_level_directory) = entries.pop() {
                        assert_eq!(
                            entries.len(),
                            0,
                            "Found more than one entry in the top level directory of the Zip file."
                        );
                        let top_level_directory = top_level_directory.unwrap();
                        let top_level_directory = top_level_directory.path();
                        assert!(
                            top_level_directory.is_dir(),
                            "The only content in the Zip is a file!"
                        );
                        create_dir_all(&final_path)
                            .expect("Could not create a directory inside the asset cache");
                        fs_extra::move_items(
                            top_level_directory
                                .read_dir()
                                .unwrap()
                                .map(|v| v.unwrap().path())
                                .collect::<Vec<PathBuf>>()
                                .as_slice(),
                            &final_path,
                            &CopyOptions::new(),
                        )
                        .unwrap();
                        // We keep the temporary directory around until we're done moving files out
                        // of it.
                        drop(temporary_directory);
                        final_path
                    } else {
                        panic!("The downloaded zip file was empty");
                    }
                } else {
                    zip_archive
                        .extract(&final_path)
                        .expect("Unable to unzip downloaded contents");
                    final_path
                }
            }
        }
    }
}
