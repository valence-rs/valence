#![doc = include_str!("../README.md")]
#![deny(
    rustdoc::broken_intra_doc_links,
    rustdoc::private_intra_doc_links,
    rustdoc::missing_crate_level_docs,
    rustdoc::invalid_codeblock_attributes,
    rustdoc::invalid_rust_codeblocks,
    rustdoc::bare_urls,
    rustdoc::invalid_html_tags
)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unused_lifetimes,
    unused_import_braces,
    unreachable_pub,
    clippy::dbg_macro
)]

use std::fs::File;
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::num::NonZeroUsize;
use std::path::PathBuf;

#[cfg(feature = "bevy")]
pub use bevy::*;
use byteorder::{BigEndian, ReadBytesExt};
use flate2::bufread::{GzDecoder, ZlibDecoder};
use lru::LruCache;
use thiserror::Error;
use tracing::warn;
use valence_nbt::Compound;

#[cfg(feature = "bevy")]
mod bevy;
#[cfg(feature = "parsing")]
pub mod parsing;

const LRU_CACHE_SIZE: NonZeroUsize = match NonZeroUsize::new(256) {
    Some(n) => n,
    None => unreachable!(),
};

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum RegionError {
    #[error("an I/O error occurred: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid chunk sector offset")]
    InvalidChunkSectorOffset,
    #[error("invalid chunk size")]
    InvalidChunkSize,
    #[error("invalid compression scheme number of {0}")]
    InvalidCompressionScheme(u8),
    #[error("failed to parse NBT: {0}")]
    Nbt(#[from] valence_nbt::binary::Error),
    #[error("not all chunk NBT data was read")]
    TrailingNbtData,
}

#[derive(Debug)]
pub struct RegionFolder {
    /// Region files. An LRU cache is used to limit the number of open file
    /// handles.
    regions: LruCache<RegionPos, RegionEntry>,
    /// Path to the "region" subdirectory in the world root.
    region_root: PathBuf,
    /// Scratch buffer for decompression.
    decompress_buf: Vec<u8>,
}

impl RegionFolder {
    pub fn new(region_root: impl Into<PathBuf>) -> Self {
        Self {
            regions: LruCache::new(LRU_CACHE_SIZE),
            region_root: region_root.into(),
            decompress_buf: Vec::new(),
        }
    }
    /// Gets the raw chunk at the given chunk position.
    ///
    /// Returns `Ok(Some(chunk))` if the chunk exists and no errors occurred
    /// loading it. Returns `Ok(None)` if the chunk does not exist and no
    /// errors occurred attempting to load it. Returns `Err(_)` if an error
    /// occurred attempting to load the chunk.
    pub fn get_chunk(&mut self, pos_x: i32, pos_z: i32) -> Result<Option<RawChunk>, RegionError> {
        let region_x = pos_x.div_euclid(32);
        let region_z = pos_z.div_euclid(32);

        let region = match self.regions.get_mut(&(region_x, region_z)) {
            Some(RegionEntry::Occupied(region)) => region,
            Some(RegionEntry::Vacant) => return Ok(None),
            None => {
                let path = self
                    .region_root
                    .join(format!("r.{region_x}.{region_z}.mca"));

                let mut file = match File::options().read(true).write(true).open(path) {
                    Ok(file) => file,
                    Err(e) if e.kind() == ErrorKind::NotFound => {
                        self.regions.put((region_x, region_z), RegionEntry::Vacant);
                        return Ok(None);
                    }
                    Err(e) => return Err(e.into()),
                };

                let mut header = [0; SECTOR_SIZE * 2];

                file.read_exact(&mut header)?;

                // TODO: this is ugly.
                let res = self.regions.get_or_insert_mut((region_x, region_z), || {
                    RegionEntry::Occupied(Region { file, header })
                });

                match res {
                    RegionEntry::Occupied(r) => r,
                    RegionEntry::Vacant => unreachable!(),
                }
            }
        };

        let chunk_idx = (pos_x.rem_euclid(32) + pos_z.rem_euclid(32) * 32) as usize;

        let location_bytes = (&region.header[chunk_idx * 4..]).read_u32::<BigEndian>()?;
        let timestamp = (&region.header[chunk_idx * 4 + SECTOR_SIZE..]).read_u32::<BigEndian>()?;

        if location_bytes == 0 {
            // No chunk exists at this position.
            return Ok(None);
        }

        let sector_offset = (location_bytes >> 8) as u64;
        let sector_count = (location_bytes & 0xff) as usize;

        // If the sector offset was <2, then the chunk data would be inside the region
        // header. That doesn't make any sense.
        if sector_offset < 2 {
            return Err(RegionError::InvalidChunkSectorOffset);
        }

        // Seek to the beginning of the chunk's data.
        region
            .file
            .seek(SeekFrom::Start(sector_offset * SECTOR_SIZE as u64))?;

        let exact_chunk_size = region.file.read_u32::<BigEndian>()? as usize;

        // size of this chunk in sectors must always be >= the exact size.
        if sector_count * SECTOR_SIZE < exact_chunk_size {
            return Err(RegionError::InvalidChunkSectorOffset);
        }

        let mut data_buf = vec![0; exact_chunk_size].into_boxed_slice();
        region.file.read_exact(&mut data_buf)?;

        let mut r = data_buf.as_ref();

        self.decompress_buf.clear();

        // What compression does the chunk use?
        let mut nbt_slice = match r.read_u8()? {
            // GZip
            1 => {
                let mut z = GzDecoder::new(r);
                z.read_to_end(&mut self.decompress_buf)?;
                self.decompress_buf.as_slice()
            }
            // Zlib
            2 => {
                let mut z = ZlibDecoder::new(r);
                z.read_to_end(&mut self.decompress_buf)?;
                self.decompress_buf.as_slice()
            }
            // Uncompressed
            3 => r,
            // Unknown
            b => return Err(RegionError::InvalidCompressionScheme(b)),
        };

        let (data, _) = Compound::from_binary(&mut nbt_slice)?;

        if !nbt_slice.is_empty() {
            return Err(RegionError::TrailingNbtData);
        }

        Ok(Some(RawChunk { data, timestamp }))
    }
}

/// A chunk represented by the raw compound data.
pub struct RawChunk {
    pub data: Compound,
    pub timestamp: u32,
}

/// X and Z positions of a region.
type RegionPos = (i32, i32);

#[allow(clippy::large_enum_variant)] // We're not moving this around.
#[derive(Debug)]
enum RegionEntry {
    /// There is a region file loaded here.
    Occupied(Region),
    /// There is no region file at this position. Don't try to read it from the
    /// filesystem again.
    Vacant,
}

#[derive(Debug)]
struct Region {
    file: File,
    /// The first 8 KiB in the file.
    header: [u8; SECTOR_SIZE * 2],
}

const SECTOR_SIZE: usize = 4096;
