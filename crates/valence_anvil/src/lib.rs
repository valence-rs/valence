#![doc = include_str!("../README.md")]

use std::fs::{DirEntry, File};
use std::hash::Hash;
use std::io::{Cursor, ErrorKind, Read, Seek, SeekFrom, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(feature = "bevy_plugin")]
pub use bevy::*;
use bitfield_struct::bitfield;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use flate2::bufread::{GzDecoder, ZlibDecoder};
use flate2::write::{GzEncoder, ZlibEncoder};
use lru::LruCache;
use thiserror::Error;
use valence_nbt::binary::{FromModifiedUtf8, ToModifiedUtf8};
use valence_nbt::Compound;

#[cfg(feature = "bevy_plugin")]
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
    #[error("failed to convert OsString")]
    OsStringConv,
    #[error("chunk is allocated, but stream is missing")]
    MissingChunkStream,
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
    #[error("oversized chunk")]
    OversizedChunk,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub enum Compression {
    Gzip = 1,
    #[default]
    Zlib = 2,
    None = 3,
}

impl Compression {
    fn from_u8(compression: u8) -> Option<Compression> {
        match compression {
            1 => Some(Compression::Gzip),
            2 => Some(Compression::Zlib),
            3 => Some(Compression::None),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, Default)]
#[non_exhaustive]
pub struct WriteOptions {
    /// Set the compression method used to write chunks. This can be useful to
    /// change in order to write anvil files compatible with older Minecraft
    /// versions.
    pub compression: Compression,

    /// Set whether to skip writing oversized chunks (>1MiB after compression).
    /// Versions older than 1.15 (19w36a) cannot read oversized chunks, so this
    /// may be useful for writing region files compatible with those
    /// versions.
    pub skip_oversized_chunks: bool,
}

#[derive(Debug)]
pub struct RegionFolder {
    /// Region files. An LRU cache is used to limit the number of open file
    /// handles.
    regions: LruCache<RegionPos, RegionEntry>,
    /// Path to the directory containing the region files and chunk files.
    region_root: PathBuf,
    /// Scratch buffer for (de)compression.
    compression_buf: Vec<u8>,
    /// Options to use for writing the chunk.
    pub write_options: WriteOptions,
}

impl RegionFolder {
    pub fn new(region_root: impl Into<PathBuf>) -> Self {
        Self {
            regions: LruCache::new(LRU_CACHE_SIZE),
            region_root: region_root.into(),
            compression_buf: Vec::new(),
            write_options: WriteOptions::default(),
        }
    }

    fn region<'a>(
        regions: &'a mut LruCache<RegionPos, RegionEntry>,
        region_root: &Path,
        region_x: i32,
        region_z: i32,
    ) -> Result<Option<&'a mut Region>, RegionError> {
        // Need to double get the entry from the cache to make the borrow checker happy.
        // Polonius will fix this eventually.
        if regions.get_mut(&(region_x, region_z)).is_some() {
            match regions.get_mut(&(region_x, region_z)) {
                Some(RegionEntry::Occupied(region)) => return Ok(Some(region)),
                Some(RegionEntry::Vacant) => return Ok(None),
                None => unreachable!(),
            }
        }

        let path = region_root.join(format!("r.{region_x}.{region_z}.mca"));

        let file = match File::options().read(true).write(true).open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == ErrorKind::NotFound => {
                regions.put((region_x, region_z), RegionEntry::Vacant);
                return Ok(None);
            }
            Err(e) => return Err(e.into()),
        };

        // TODO: this is ugly.
        // TODO: try_get_or_insert_mut
        regions.try_get_or_insert((region_x, region_z), || {
            Region::open(file).map(|region| RegionEntry::Occupied(Box::new(region)))
        })?;
        let Some(RegionEntry::Occupied(res)) = regions.get_mut(&(region_x, region_z)) else {
            unreachable!()
        };
        Ok(Some(res))
    }

    /// Gets the raw chunk at the given chunk position.
    ///
    /// Returns `Ok(Some(chunk))` if the chunk exists and no errors occurred
    /// loading it. Returns `Ok(None)` if the chunk does not exist and no
    /// errors occurred attempting to load it. Returns `Err(_)` if an error
    /// occurred attempting to load the chunk.
    pub fn get_chunk<S>(
        &mut self,
        pos_x: i32,
        pos_z: i32,
    ) -> Result<Option<RawChunk<S>>, RegionError>
    where
        S: for<'a> FromModifiedUtf8<'a> + Hash + Ord,
    {
        let region_x = pos_x.div_euclid(32);
        let region_z = pos_z.div_euclid(32);

        let Some(region) = Self::region(&mut self.regions, &self.region_root, region_x, region_z)?
        else {
            return Ok(None);
        };

        region.get_chunk(pos_x, pos_z, &mut self.compression_buf, &self.region_root)
    }

    /// Deletes the chunk at the given chunk position, returning whether the
    /// chunk existed before it was deleted.
    ///
    /// Note that this only marks the chunk as deleted so that it cannot be
    /// retrieved, and can be overwritten by other chunks later. It does not
    /// decrease the size of the region file.
    pub fn delete_chunk(&mut self, pos_x: i32, pos_z: i32) -> Result<bool, RegionError> {
        let region_x = pos_x.div_euclid(32);
        let region_z = pos_z.div_euclid(32);

        let Some(region) = Self::region(&mut self.regions, &self.region_root, region_x, region_z)?
        else {
            return Ok(false);
        };

        region.delete_chunk(pos_x, pos_z, true, &self.region_root)
    }

    /// Sets the raw chunk at the given position, overwriting the old chunk if
    /// it exists.
    pub fn set_chunk<S>(
        &mut self,
        pos_x: i32,
        pos_z: i32,
        chunk: &Compound<S>,
    ) -> Result<(), RegionError>
    where
        S: ToModifiedUtf8 + Hash + Ord,
    {
        let region_x = pos_x.div_euclid(32);
        let region_z = pos_z.div_euclid(32);

        let region = match Self::region(&mut self.regions, &self.region_root, region_x, region_z)? {
            Some(region) => region,
            None => {
                let path = self
                    .region_root
                    .join(format!("r.{region_x}.{region_z}.mca"));

                let file = File::options()
                    .read(true)
                    .write(true)
                    .create(true)
                    .open(path)?;

                // TODO: try_get_or_insert_mut
                self.regions.put(
                    (region_x, region_z),
                    RegionEntry::Occupied(Box::new(Region::create(file)?)),
                );
                let Some(RegionEntry::Occupied(region)) =
                    self.regions.get_mut(&(region_x, region_z))
                else {
                    unreachable!()
                };
                region
            }
        };

        region.set_chunk(
            pos_x,
            pos_z,
            chunk,
            self.write_options,
            &mut self.compression_buf,
            &self.region_root,
        )
    }

    /// Returns an iterator over all existing chunks in all regions.
    pub fn all_chunk_positions(
        &mut self,
    ) -> Result<impl Iterator<Item = Result<(i32, i32), RegionError>> + '_, RegionError> {
        fn extract_region_coordinates(
            file: std::io::Result<DirEntry>,
        ) -> Result<Option<(i32, i32)>, RegionError> {
            let file = file?;

            if !file.file_type()?.is_file() {
                return Ok(None);
            }

            let file_name = file
                .file_name()
                .into_string()
                .map_err(|_| RegionError::OsStringConv)?;

            // read the file name as r.x.z.mca
            let mut split = file_name.splitn(4, '.');
            if split.next() != Some("r") {
                return Ok(None);
            }
            let Some(Ok(x)) = split.next().map(str::parse) else {
                return Ok(None);
            };
            let Some(Ok(z)) = split.next().map(str::parse) else {
                return Ok(None);
            };
            if split.next() != Some("mca") {
                return Ok(None);
            }

            Ok(Some((x, z)))
        }

        fn region_chunks(
            this: &mut RegionFolder,
            pos: Result<(i32, i32), RegionError>,
        ) -> impl Iterator<Item = Result<(i32, i32), RegionError>> {
            let positions = match pos {
                Ok((region_x, region_z)) => {
                    match RegionFolder::region(
                        &mut this.regions,
                        &this.region_root,
                        region_x,
                        region_z,
                    ) {
                        Ok(Some(region)) => region.chunk_positions(region_x, region_z),
                        Ok(None) => Vec::new(),
                        Err(err) => vec![Err(err)],
                    }
                }
                Err(err) => vec![Err(err)],
            };
            positions.into_iter()
        }

        Ok(std::fs::read_dir(&self.region_root)?
            .filter_map(|file| extract_region_coordinates(file).transpose())
            .flat_map(|pos| region_chunks(self, pos)))
    }
}

/// A chunk represented by the raw compound data.
pub struct RawChunk<S = String> {
    pub data: Compound<S>,
    pub timestamp: u32,
}

/// X and Z positions of a region.
type RegionPos = (i32, i32);

#[derive(Debug)]
enum RegionEntry {
    /// There is a region file loaded here.
    Occupied(Box<Region>),
    /// There is no region file at this position. Don't try to read it from the
    /// filesystem again.
    Vacant,
}

#[bitfield(u32)]
struct Location {
    count: u8,
    #[bits(24)]
    offset: u32,
}

impl Location {
    fn is_none(self) -> bool {
        self.0 == 0
    }

    fn offset_and_count(self) -> (u64, usize) {
        (self.offset() as u64, self.count() as usize)
    }
}

#[derive(Debug)]
struct Region {
    file: File,
    locations: [Location; 1024],
    timestamps: [u32; 1024],
    used_sectors: bitvec::vec::BitVec,
}

impl Region {
    fn create(mut file: File) -> Result<Self, RegionError> {
        let header = [0; SECTOR_SIZE * 2];
        file.write_all(&header)?;

        Ok(Self {
            file,
            locations: [Location::default(); 1024],
            timestamps: [0; 1024],
            used_sectors: bitvec::vec::BitVec::repeat(true, 2),
        })
    }

    fn open(mut file: File) -> Result<Self, RegionError> {
        let mut header = [0; SECTOR_SIZE * 2];
        file.read_exact(&mut header)?;

        let locations = std::array::from_fn(|i| {
            Location(u32::from_be_bytes(
                header[i * 4..i * 4 + 4].try_into().unwrap(),
            ))
        });
        let timestamps = std::array::from_fn(|i| {
            u32::from_be_bytes(
                header[i * 4 + SECTOR_SIZE..i * 4 + SECTOR_SIZE + 4]
                    .try_into()
                    .unwrap(),
            )
        });

        let mut used_sectors = bitvec::vec::BitVec::repeat(true, 2);
        for location in locations {
            if location.is_none() {
                // No chunk exists at this position.
                continue;
            }

            let (sector_offset, sector_count) = location.offset_and_count();
            if sector_offset < 2 {
                // skip locations pointing inside the header
                continue;
            }
            if sector_count == 0 {
                continue;
            }
            if sector_offset * SECTOR_SIZE as u64 > file.metadata()?.len() {
                // this would go past the end of the file, which is impossible
                continue;
            }

            Self::reserve_sectors(&mut used_sectors, sector_offset, sector_count);
        }

        Ok(Self {
            file,
            locations,
            timestamps,
            used_sectors,
        })
    }

    fn get_chunk<S>(
        &mut self,
        pos_x: i32,
        pos_z: i32,
        decompress_buf: &mut Vec<u8>,
        region_root: &Path,
    ) -> Result<Option<RawChunk<S>>, RegionError>
    where
        S: for<'a> FromModifiedUtf8<'a> + Hash + Ord,
    {
        let chunk_idx = Self::chunk_idx(pos_x, pos_z);

        let location = self.locations[chunk_idx];
        let timestamp = self.timestamps[chunk_idx];

        if location.is_none() {
            // No chunk exists at this position.
            return Ok(None);
        }

        let (sector_offset, sector_count) = location.offset_and_count();

        // If the sector offset was <2, then the chunk data would be inside the region
        // header. That doesn't make any sense.
        if sector_offset < 2 {
            return Err(RegionError::InvalidChunkSectorOffset);
        }

        // Seek to the beginning of the chunk's data.
        self.file
            .seek(SeekFrom::Start(sector_offset * SECTOR_SIZE as u64))?;

        let exact_chunk_size = self.file.read_u32::<BigEndian>()? as usize;
        if exact_chunk_size == 0 {
            return Err(RegionError::MissingChunkStream);
        }

        // size of this chunk in sectors must always be >= the exact size.
        if sector_count * SECTOR_SIZE < exact_chunk_size {
            return Err(RegionError::InvalidChunkSize);
        }

        let mut compression = self.file.read_u8()?;

        let data_buf = if Self::is_external_stream_chunk(compression) {
            compression = Self::external_chunk_version(compression);
            let mut external_file =
                File::open(Self::external_chunk_file(pos_x, pos_z, region_root))?;
            let mut buf = Vec::new();
            external_file.read_to_end(&mut buf)?;
            buf.into_boxed_slice()
        } else {
            // the size includes the version of the stream, but we have already read that
            let mut data_buf = vec![0; exact_chunk_size - 1].into_boxed_slice();
            self.file.read_exact(&mut data_buf)?;
            data_buf
        };

        let r = data_buf.as_ref();

        decompress_buf.clear();

        // What compression does the chunk use?
        let mut nbt_slice = match Compression::from_u8(compression) {
            Some(Compression::Gzip) => {
                let mut z = GzDecoder::new(r);
                z.read_to_end(decompress_buf)?;
                decompress_buf.as_slice()
            }
            Some(Compression::Zlib) => {
                let mut z = ZlibDecoder::new(r);
                z.read_to_end(decompress_buf)?;
                decompress_buf.as_slice()
            }
            // Uncompressed
            Some(Compression::None) => r,
            // Unknown
            None => return Err(RegionError::InvalidCompressionScheme(compression)),
        };

        let (data, _) = valence_nbt::from_binary(&mut nbt_slice)?;

        if !nbt_slice.is_empty() {
            return Err(RegionError::TrailingNbtData);
        }

        Ok(Some(RawChunk { data, timestamp }))
    }

    fn delete_chunk(
        &mut self,
        pos_x: i32,
        pos_z: i32,
        delete_on_disk: bool,
        region_root: &Path,
    ) -> Result<bool, RegionError> {
        let chunk_idx = Self::chunk_idx(pos_x, pos_z);

        let location = self.locations[chunk_idx];
        if location.is_none() {
            // chunk already missing, nothing to delete
            return Ok(false);
        }

        if delete_on_disk {
            self.file.seek(SeekFrom::Start(chunk_idx as u64 * 4))?;
            self.file.write_u32::<BigEndian>(0)?;

            Self::delete_external_chunk_file(pos_x, pos_z, region_root)?;
        }

        let (sector_offset, sector_count) = location.offset_and_count();
        if sector_offset >= 2 {
            let start_index = sector_offset as usize;
            let end_index = start_index + sector_count;
            let len = self.used_sectors.len();
            self.used_sectors[start_index.min(len)..end_index.min(len)].fill(false);
        }

        self.locations[chunk_idx] = Location::new();

        Ok(true)
    }

    fn set_chunk<S>(
        &mut self,
        pos_x: i32,
        pos_z: i32,
        chunk: &Compound<S>,
        options: WriteOptions,
        compress_buf: &mut Vec<u8>,
        region_root: &Path,
    ) -> Result<(), RegionError>
    where
        S: ToModifiedUtf8 + Hash + Ord,
    {
        // erase the chunk from allocated chunks (not from disk)
        self.delete_chunk(pos_x, pos_z, false, region_root)?;

        // write the chunk into NBT and compress it according to the compression method
        compress_buf.clear();
        let mut compress_cursor = Cursor::new(compress_buf);
        match options.compression {
            Compression::Gzip => valence_nbt::to_binary(
                chunk,
                GzEncoder::new(&mut compress_cursor, flate2::Compression::default()),
                "",
            )?,
            Compression::Zlib => valence_nbt::to_binary(
                chunk,
                ZlibEncoder::new(&mut compress_cursor, flate2::Compression::default()),
                "",
            )?,
            Compression::None => valence_nbt::to_binary(chunk, &mut compress_cursor, "")?,
        }
        let compress_buf = compress_cursor.into_inner();

        // additional 5 bytes for exact chunk size + compression type, then add
        // SECTOR_SIZE - 1 for rounding up
        let num_sectors_needed = (compress_buf.len() + 5 + SECTOR_SIZE - 1) / SECTOR_SIZE;
        let (start_sector, num_sectors) = if num_sectors_needed >= 256 {
            if options.skip_oversized_chunks {
                return Err(RegionError::OversizedChunk);
            }

            // write oversized chunk to external file
            File::create(Self::external_chunk_file(pos_x, pos_z, region_root))?
                .write_all(&*compress_buf)?;

            let start_sector = self.allocate_sectors(1);
            self.file
                .seek(SeekFrom::Start(start_sector * SECTOR_SIZE as u64))?;

            // write the exact chunk size, which includes *only* the compression version
            // (the rest of the chunk is external)
            self.file.write_u32::<BigEndian>(1)?;
            // write the compression, with the marker which says our chunk is oversized
            self.file.write_u8((options.compression as u8) | 0x80)?;

            (start_sector, 1)
        } else {
            // delete the oversized chunk if it existed before
            Self::delete_external_chunk_file(pos_x, pos_z, region_root)?;

            let start_sector = self.allocate_sectors(num_sectors_needed);
            self.file
                .seek(SeekFrom::Start(start_sector * SECTOR_SIZE as u64))?;

            // write the exact chunk size, which accounts for the compression version which
            // is not in our compress_buf
            self.file
                .write_u32::<BigEndian>((compress_buf.len() + 1) as u32)?;
            // write the compression
            self.file.write_u8(options.compression as u8)?;
            // write the data
            self.file.write_all(&*compress_buf)?;

            (start_sector, num_sectors_needed)
        };

        let location = Location::new()
            .with_offset(start_sector as u32)
            .with_count(num_sectors as u8);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs() as u32)
            .unwrap_or(0);

        // write changed header information to file
        let chunk_idx = Self::chunk_idx(pos_x, pos_z);
        self.file.seek(SeekFrom::Start(chunk_idx as u64 * 4))?;
        self.file.write_u32::<BigEndian>(location.0)?;
        self.file
            .seek(SeekFrom::Start(chunk_idx as u64 * 4 + SECTOR_SIZE as u64))?;
        self.file.write_u32::<BigEndian>(timestamp)?;

        // write changed header information to our header
        self.locations[chunk_idx] = location;
        self.timestamps[chunk_idx] = timestamp;

        // pad file to multiple of SECTOR_SIZE
        let file_length = self.file.seek(SeekFrom::End(0))?;
        let rem = file_length as usize % SECTOR_SIZE;
        if rem != 0 {
            self.file
                .write_all(&[0; SECTOR_SIZE][..SECTOR_SIZE - rem])?;
        }

        Ok(())
    }

    fn chunk_positions(
        &self,
        region_x: i32,
        region_z: i32,
    ) -> Vec<Result<(i32, i32), RegionError>> {
        self.locations
            .iter()
            .enumerate()
            .filter_map(move |(index, location)| {
                if location.is_none() {
                    None
                } else {
                    Some((
                        region_x * 32 + (index % 32) as i32,
                        region_z * 32 + (index / 32) as i32,
                    ))
                }
            })
            .map(Ok)
            .collect()
    }

    fn external_chunk_file(pos_x: i32, pos_z: i32, region_root: &Path) -> PathBuf {
        region_root
            .to_path_buf()
            .join(format!("c.{pos_x}.{pos_z}.mcc"))
    }

    fn delete_external_chunk_file(
        pos_x: i32,
        pos_z: i32,
        region_root: &Path,
    ) -> Result<(), RegionError> {
        match std::fs::remove_file(Self::external_chunk_file(pos_x, pos_z, region_root)) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err.into()),
        }
    }

    fn reserve_sectors(
        used_sectors: &mut bitvec::vec::BitVec,
        sector_offset: u64,
        sector_count: usize,
    ) {
        let start_index = sector_offset as usize;
        let end_index = sector_offset as usize + sector_count;
        if used_sectors.len() < end_index {
            used_sectors.resize(start_index, false);
            used_sectors.resize(end_index, true);
        } else {
            used_sectors[start_index..end_index].fill(true);
        }
    }

    fn allocate_sectors(&mut self, num_sectors: usize) -> u64 {
        // find the first set of consecutive free sectors of length num_sectors
        let mut index = 0;
        let free_space_start = loop {
            let Some(mut free_space_start) = self.used_sectors[index..].first_zero() else {
                // we have reached a sequence of 1's at the end of the list, so next free space
                // is at the end of the file
                break self.used_sectors.len();
            };
            free_space_start += index;

            let Some(mut free_space_end) = self.used_sectors[free_space_start..].first_one() else {
                // there is no 1 after this 0, so we have enough space here (even if we have to
                // increase the file size)
                break free_space_start;
            };
            free_space_end += free_space_start;

            if free_space_end - free_space_start >= num_sectors {
                // if the free space end is far enough from the free space start, we have enough
                // space
                break free_space_start;
            }

            index = free_space_end;
        };

        Self::reserve_sectors(&mut self.used_sectors, free_space_start as u64, num_sectors);
        free_space_start as u64
    }

    fn chunk_idx(pos_x: i32, pos_z: i32) -> usize {
        (pos_x.rem_euclid(32) + pos_z.rem_euclid(32) * 32) as usize
    }

    fn is_external_stream_chunk(stream_version: u8) -> bool {
        (stream_version & 0x80) != 0
    }

    fn external_chunk_version(stream_version: u8) -> u8 {
        stream_version & !0x80
    }
}

const SECTOR_SIZE: usize = 4096;
