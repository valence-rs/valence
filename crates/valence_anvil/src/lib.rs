use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::fs::File;
use std::io;
use std::io::{ErrorKind, Read, Seek, SeekFrom};
use std::path::PathBuf;

use byteorder::{BigEndian, ReadBytesExt};
use flate2::bufread::{GzDecoder, ZlibDecoder};
use thiserror::Error;
#[cfg(feature = "valence")]
pub use to_valence::*;
use valence_nbt::Compound;

#[cfg(feature = "valence")]
mod to_valence;

#[derive(Debug)]
pub struct AnvilWorld {
    /// Path to the "region" subdirectory in the world root.
    region_root: PathBuf,
    // TODO: LRU cache for region file handles.
    /// Maps region (x, z) positions to region files.
    regions: BTreeMap<(i32, i32), Region>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct AnvilChunk {
    /// This chunk's NBT data.
    pub data: Compound,
    /// The time this chunk was last modified measured in seconds since the
    /// epoch.
    pub timestamp: u32,
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ReadChunkError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Nbt(#[from] valence_nbt::Error),
    #[error("invalid chunk sector offset")]
    BadSectorOffset,
    #[error("invalid chunk size")]
    BadChunkSize,
    #[error("unknown compression scheme number of {0}")]
    UnknownCompressionScheme(u8),
    #[error("not all chunk NBT data was read")]
    IncompleteNbtRead,
}

#[derive(Debug)]
struct Region {
    file: File,
    /// The first 8 KiB in the file.
    header: [u8; SECTOR_SIZE * 2],
}

const SECTOR_SIZE: usize = 4096;

impl AnvilWorld {
    pub fn new(world_root: impl Into<PathBuf>) -> Self {
        let mut region_root = world_root.into();
        region_root.push("region");

        Self {
            region_root,
            regions: BTreeMap::new(),
        }
    }

    /// Reads a chunk from the file system with the given chunk coordinates. If
    /// no chunk exists at the position, then `None` is returned.
    pub fn read_chunk(
        &mut self,
        chunk_x: i32,
        chunk_z: i32,
    ) -> Result<Option<AnvilChunk>, ReadChunkError> {
        let region_x = chunk_x.div_euclid(32);
        let region_z = chunk_z.div_euclid(32);

        let region = match self.regions.entry((region_x, region_z)) {
            Entry::Vacant(ve) => {
                // Load the region file if it exists. Otherwise, the chunk is considered absent.

                // TODO: Add tombstone for missing region file in `regions`.

                let path = self
                    .region_root
                    .join(format!("r.{region_x}.{region_z}.mca"));

                let mut file = match File::options().read(true).write(true).open(path) {
                    Ok(file) => file,
                    Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
                    Err(e) => return Err(e.into()),
                };

                let mut header = [0; SECTOR_SIZE * 2];

                file.read_exact(&mut header)?;

                ve.insert(Region { file, header })
            }
            Entry::Occupied(oe) => oe.into_mut(),
        };

        let chunk_idx = (chunk_x.rem_euclid(32) + chunk_z.rem_euclid(32) * 32) as usize;

        let location_bytes = (&region.header[chunk_idx * 4..]).read_u32::<BigEndian>()?;
        let timestamp = (&region.header[chunk_idx * 4 + SECTOR_SIZE..]).read_u32::<BigEndian>()?;

        if location_bytes == 0 {
            // No chunk exists at this position.
            return Ok(None);
        }

        let sector_offset = (location_bytes >> 8) as u64;
        let sector_count = (location_bytes & 0xff) as usize;

        if sector_offset < 2 {
            // If the sector offset was <2, then the chunk data would be inside the region
            // header. That doesn't make any sense.
            return Err(ReadChunkError::BadSectorOffset);
        }

        // Seek to the beginning of the chunk's data.
        region
            .file
            .seek(SeekFrom::Start(sector_offset * SECTOR_SIZE as u64))?;

        let exact_chunk_size = region.file.read_u32::<BigEndian>()? as usize;

        if exact_chunk_size > sector_count * SECTOR_SIZE {
            // Sector size of this chunk must always be >= the exact size.
            return Err(ReadChunkError::BadChunkSize);
        }

        let mut data_buf = vec![0; exact_chunk_size].into_boxed_slice();
        region.file.read_exact(&mut data_buf)?;

        let mut r = data_buf.as_ref();

        let mut decompress_buf = vec![];

        // What compression does the chunk use?
        let mut nbt_slice = match r.read_u8()? {
            // GZip
            1 => {
                let mut z = GzDecoder::new(r);
                z.read_to_end(&mut decompress_buf)?;
                decompress_buf.as_slice()
            }
            // Zlib
            2 => {
                let mut z = ZlibDecoder::new(r);
                z.read_to_end(&mut decompress_buf)?;
                decompress_buf.as_slice()
            }
            // Uncompressed
            3 => r,
            // Unknown
            b => return Err(ReadChunkError::UnknownCompressionScheme(b)),
        };

        let (data, _) = valence_nbt::from_binary_slice(&mut nbt_slice)?;

        if !nbt_slice.is_empty() {
            return Err(ReadChunkError::IncompleteNbtRead);
        }

        Ok(Some(AnvilChunk { data, timestamp }))
    }
}
