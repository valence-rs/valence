use std::fmt::{self, Debug, Formatter};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use byteorder::{BigEndian, ByteOrder, ReadBytesExt};
use valence::chunk::{ChunkPos, UnloadedChunk};

use crate::chunk::parse_chunk_nbt;
use crate::compression::CompressionScheme;
use crate::error::{DataFormatError, Error};
use crate::AnvilWorldConfig;

#[derive(Debug)]
pub struct Region<S> {
    source: S,
    offset: u64,
    position: RegionPos,
    header: AnvilHeader,
}

impl Region<File> {
    /// Convenience method, creates a Region object from the given file and
    /// position.
    pub fn from_file(source: File, position: RegionPos) -> Result<Self, std::io::Error> {
        Self::from_seek(source, 0, position)
    }
}

impl<S: Read + Seek> Region<S> {
    /// Creates a Region object using the incoming stream. The offset defines
    /// the position of the header start.
    pub fn from_seek(
        mut source: S,
        offset: u64,
        position: RegionPos,
    ) -> Result<Self, std::io::Error> {
        source.seek(SeekFrom::Start(offset))?;
        let header = AnvilHeader::parse(&mut source)?;

        Ok(Self {
            source,
            offset,
            position,
            header,
        })
    }

    /// Get the last time the chunk was modified in seconds since epoch.
    pub fn chunk_timestamp(&self, chunk_pos: ChunkPos) -> Option<ChunkTimestamp> {
        self.header
            .timestamp((chunk_pos.x & 31) as usize, (chunk_pos.z & 31) as usize)
            .into_option()
    }

    fn read_chunk_bytes(&mut self, chunk_pos: ChunkPos) -> Result<Option<Vec<u8>>, Error> {
        let seek_pos = self
            .header
            .offset((chunk_pos.x & 31) as usize, (chunk_pos.z & 31) as usize);

        self.source
            .seek(SeekFrom::Start(seek_pos.offset() + self.offset))?;

        if seek_pos.len() == 0 {
            return Ok(None);
        }

        let compressed_chunk_size = {
            let mut buf = [0u8; 4];
            self.source.read_exact(&mut buf)?;
            BigEndian::read_u32(&buf) as usize
        };

        if compressed_chunk_size == 0 {
            return Err(Error::DataFormatError(DataFormatError::InvalidChunkSize(
                compressed_chunk_size,
            )));
        }

        let compression = CompressionScheme::from_raw(self.source.read_u8()?)?;
        let uncompressed_buffer =
            compression.read_to_vec(&mut self.source, compressed_chunk_size - 1)?;
        Ok(Some(uncompressed_buffer))
    }

    pub(crate) fn parse_chunks<I: IntoIterator<Item = ChunkPos>>(
        &mut self,
        world_config: &AnvilWorldConfig,
        positions: I,
    ) -> Result<impl Iterator<Item = (ChunkPos, Option<UnloadedChunk>)>, Error> {
        let mut results = Vec::<(ChunkPos, Option<UnloadedChunk>)>::new();

        for pos in positions.into_iter() {
            assert!(
                self.position.contains(pos),
                "Chunk position {:?} was not found in region {:?}",
                pos,
                self.position
            );

            let chunk_data = self.read_chunk_bytes(pos)?;
            if let Some(chunk_data) = chunk_data {
                let nbt = valence::nbt::from_binary_slice(&mut chunk_data.as_slice())?.0;
                match parse_chunk_nbt(nbt, world_config) {
                    Err(Error::DataFormatError(DataFormatError::MissingChunkNBT { .. }))
                    | Err(Error::DataFormatError(DataFormatError::UnexpectedChunkState(..))) => {
                        // The chunk is missing vital data and cannot be parsed.
                        results.push((pos, None));
                    }
                    Err(e) => return Err(e),
                    Ok(parsed_chunk) => {
                        results.push((pos, Some(parsed_chunk)));
                    }
                }
            } else {
                results.push((pos, None));
            }
        }

        Ok(results.into_iter())
    }
}

#[derive(Copy, Clone, Debug)]
struct AnvilHeader {
    offsets: [ChunkSeekLocation; 1024],
    timestamps: [ChunkTimestamp; 1024],
}

impl AnvilHeader {
    /// Parses the header bytes from the current position
    fn parse<R: Read>(source: &mut R) -> Result<Self, std::io::Error> {
        let mut offsets = [ChunkSeekLocation::zero(); 1024];
        for offset in &mut offsets {
            let mut buf = [0u8; 4];
            source.read_exact(&mut buf)?;
            offset.load(buf);
        }
        let mut timestamps = [ChunkTimestamp::zero(); 1024];
        for timestamp in &mut timestamps {
            let mut buf = [0u8; 4];
            source.read_exact(&mut buf)?;
            timestamp.load(buf);
        }
        Ok(Self {
            offsets,
            timestamps,
        })
    }

    #[inline(always)]
    fn offset(&self, x: usize, z: usize) -> &ChunkSeekLocation {
        &self.offsets[(x & 0b11111) + ((z & 0b11111) * 32)]
    }

    #[inline(always)]
    fn timestamp(&self, x: usize, z: usize) -> &ChunkTimestamp {
        &self.timestamps[(x & 0b11111) + ((z & 0b11111) * 32)]
    }
}

/// The location of the chunk inside the region file.
#[derive(Copy, Clone, Debug)]
struct ChunkSeekLocation {
    offset_sectors: u32,
    len_sectors: u8,
}

impl ChunkSeekLocation {
    const fn zero() -> Self {
        Self {
            offset_sectors: 0,
            len_sectors: 0,
        }
    }

    const fn offset(&self) -> u64 {
        self.offset_sectors as u64 * 1024 * 4
    }

    const fn len(&self) -> usize {
        self.len_sectors as usize * 1024 * 4
    }

    fn load(&mut self, chunk: [u8; 4]) {
        self.offset_sectors = BigEndian::read_u24(&chunk[..3]);
        self.len_sectors = chunk[3];
    }
}

/// The timestamp when the chunk was last modified in seconds since epoch.
#[derive(Copy, Clone)]
pub struct ChunkTimestamp(u32);

impl Debug for ChunkTimestamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}s", self.0)
    }
}

impl ChunkTimestamp {
    const fn zero() -> Self {
        Self(0)
    }

    fn load(&mut self, chunk: [u8; 4]) {
        self.0 = BigEndian::read_u32(&chunk)
    }

    fn into_option(self) -> Option<Self> {
        if self.0 == 0 {
            None
        } else {
            Some(self)
        }
    }

    #[inline(always)]
    pub fn seconds_since_epoch(self) -> u32 {
        self.0
    }
}

#[derive(Copy, Clone, Debug, PartialOrd, PartialEq, Eq, Ord)]
pub struct RegionPos {
    x: i32,
    z: i32,
}

impl From<ChunkPos> for RegionPos {
    fn from(pos: ChunkPos) -> Self {
        Self {
            x: pos.x >> 5,
            z: pos.z >> 5,
        }
    }
}

impl RegionPos {
    pub fn path(self, world_root: impl AsRef<Path>) -> PathBuf {
        world_root
            .as_ref()
            .join("region")
            .join(format!("r.{}.{}.mca", self.x, self.z))
    }

    pub fn contains(self, chunk_pos: ChunkPos) -> bool {
        Self::from(chunk_pos) == self
    }
}
