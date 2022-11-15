use std::fmt::{self, Debug, Formatter};
use std::io::SeekFrom;
use std::path::{Path, PathBuf};

use byteorder::{BigEndian, ByteOrder};
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt};
use tokio::sync::Mutex;
use valence::biome::BiomeId;
use valence::chunk::{Chunk, ChunkPos, UnloadedChunk};
use valence::nbt::{Compound, List, Value};
use valence::protocol::block::{BlockKind, BlockState, PropName, PropValue};
use valence::protocol::Ident;

use crate::compression::CompressionScheme;
use crate::error::{DataFormatError, Error, NbtFormatError};
use crate::palette::DataFormat;
use crate::{palette, AnvilWorld};

#[derive(Debug)]
pub struct Region<S> {
    source: Mutex<S>,
    offset: u64,
    position: RegionPos,
    header: AnvilHeader,
}

impl Region<File> {
    /// Convenience method, creates a Region object from the given file and
    /// position.
    pub async fn from_file(source: File, position: RegionPos) -> Result<Self, std::io::Error> {
        Self::from_seek(Mutex::new(source), 0, position).await
    }
}

impl<S: AsyncRead + AsyncSeek + Unpin> Region<S> {
    /// Creates a Region object using the incoming stream. The offset defines
    /// the position of the header start.
    pub async fn from_seek(
        source: Mutex<S>,
        offset: u64,
        position: RegionPos,
    ) -> Result<Self, std::io::Error> {
        let mut lock = source.lock().await;
        lock.seek(SeekFrom::Start(offset)).await?;
        let header = AnvilHeader::parse(&mut *lock).await?;
        drop(lock);

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

    async fn read_chunk_bytes(&self, chunk_pos: ChunkPos) -> Result<Option<Vec<u8>>, Error> {
        let seek_pos = self
            .header
            .offset((chunk_pos.x & 31) as usize, (chunk_pos.z & 31) as usize);

        let mut lock = self.source.lock().await;

        lock.seek(SeekFrom::Start(seek_pos.offset() + self.offset))
            .await?;

        if seek_pos.len() == 0 {
            return Ok(None);
        }

        let compressed_chunk_size = {
            let mut buf = [0u8; 4];
            lock.read_exact(&mut buf).await?;
            BigEndian::read_u32(&buf) as usize
        };

        if compressed_chunk_size == 0 {
            return Err(Error::DataFormatError(DataFormatError::InvalidChunkSize(
                compressed_chunk_size,
            )));
        }

        let compression = CompressionScheme::from_raw(lock.read_u8().await?)?;
        let uncompressed_buffer = compression
            .read_to_vec(&mut *lock, compressed_chunk_size - 1)
            .await?;
        Ok(Some(uncompressed_buffer))
    }

    pub(crate) async fn parse_chunks<I: IntoIterator<Item = ChunkPos>>(
        &self,
        world: &AnvilWorld,
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

            let chunk_data = self.read_chunk_bytes(pos).await?;
            if let Some(chunk_data) = chunk_data {
                let mut nbt = valence::nbt::from_binary_slice(&mut chunk_data.as_slice())?.0;
                match Self::parse_chunk_nbt(&mut nbt, world) {
                    Err(Error::DataFormatError(DataFormatError::InvalidChunkState(..))) => {
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

    //TODO: This function is very large and should be separated into dedicated
    // functions at some point.
    fn parse_chunk_nbt(nbt: &mut Compound, world: &AnvilWorld) -> Result<UnloadedChunk, Error> {
        fn take_assume<R>(compound: &mut Compound, key: &'static str) -> Result<R, Error>
        where
            Option<R>: From<Value>,
        {
            match compound.remove(key) {
                None => Err(Error::NbtFormatError(NbtFormatError::MissingKey(
                    key.to_string(),
                ))),
                Some(value) => {
                    if let Some(value) = Option::<R>::from(value) {
                        Ok(value)
                    } else {
                        Err(Error::NbtFormatError(NbtFormatError::InvalidType(
                            key.to_string(),
                        )))
                    }
                }
            }
        }

        fn take_assume_optional<R>(compound: &mut Compound, key: &'static str) -> Option<R>
        where
            Option<R>: From<Value>,
        {
            match compound.remove(key) {
                None => None,
                Some(value) => Option::<R>::from(value),
            }
        }

        let status: String = take_assume(nbt, "Status")?;
        if status.as_str() != "full" {
            return Err(Error::DataFormatError(DataFormatError::InvalidChunkState(
                status,
            )));
        }

        if let Some(Value::List(List::Compound(nbt_sections))) = nbt.remove("sections") {
            let mut y_max = 0i8;
            let mut y_min = 0i8;

            for chunk_nbt in nbt_sections.iter() {
                if let Some(Value::Byte(section_y)) = chunk_nbt.get("Y") {
                    y_max = y_max.max(*section_y);
                    y_min = y_min.min(*section_y);
                } else {
                    return Err(Error::NbtFormatError(NbtFormatError::MissingKey(
                        "Y".to_string(),
                    )));
                }
            }

            // `y_max` should always be equal or higher than `y_min`. Therefore,
            // section_height is positive.
            let section_height = ((y_max as isize - y_min as isize) as usize * 16) + 16;
            let y_raise = isize::from(-y_min) * 16;

            // Parsing sections
            let mut chunk = UnloadedChunk::new(section_height);
            for mut nbt_section in nbt_sections.into_iter() {
                let chunk_y_offset: isize =
                    isize::from(take_assume::<i8>(&mut nbt_section, "Y")?) * 16;

                // Block states
                let mut nbt_block_states: Compound = take_assume(&mut nbt_section, "block_states")?;
                let parsed_block_state_palette: Vec<BlockState> =
                    if let Some(Value::List(List::Compound(nbt_palette_vec))) =
                        nbt_block_states.remove("palette")
                    {
                        let mut palette_vec: Vec<BlockState> =
                            Vec::with_capacity(nbt_palette_vec.len());
                        for mut nbt_palette in nbt_palette_vec {
                            let block_id =
                                Ident::new(take_assume::<String>(&mut nbt_palette, "Name")?)?;
                            let block_kind =
                                if let Some(block_kind) = BlockKind::from_str(block_id.path()) {
                                    block_kind
                                } else {
                                    return Err(Error::DataFormatError(
                                        DataFormatError::UnknownType(block_id),
                                    ));
                                };
                            let mut block_state = BlockState::from_kind(block_kind);
                            if let Some(Value::Compound(nbt_palette_properties)) =
                                nbt_palette.remove("Properties")
                            {
                                for (property_name_raw, property_value) in nbt_palette_properties {
                                    if let Value::String(property_value) = property_value {
                                        let property_name = PropName::from_str(&property_name_raw);
                                        let property_value = PropValue::from_str(&property_value);
                                        if let (Some(property_name), Some(property_value)) =
                                            (property_name, property_value)
                                        {
                                            block_state =
                                                block_state.set(property_name, property_value);
                                        } else {
                                            return Err(Error::NbtFormatError(
                                                NbtFormatError::MissingKey(property_name_raw),
                                            ));
                                        }
                                    } else {
                                        return Err(Error::NbtFormatError(
                                            NbtFormatError::InvalidType(property_name_raw),
                                        ));
                                    }
                                }
                            }
                            palette_vec.push(block_state);
                        }
                        palette_vec
                    } else {
                        return Err(Error::NbtFormatError(NbtFormatError::InvalidType(
                            "palette".to_string(),
                        )));
                    };

                // Block state palette
                palette::parse_palette::<BlockState, _>(
                    &parsed_block_state_palette,
                    take_assume_optional(&mut nbt_block_states, "data"),
                    4,
                    16 * 16 * 16,
                    &mut |data| {
                        match data {
                            DataFormat::All(state) => {
                                if !state.is_air() {
                                    for x in 0..16 {
                                        for y in 0..16isize {
                                            for z in 0..16 {
                                                chunk.set_block_state(
                                                    x,
                                                    (y + chunk_y_offset + y_raise) as usize,
                                                    z,
                                                    state,
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                            DataFormat::Palette(index, state) => {
                                let y = (index >> 8 & 0b1111) as isize;
                                let z = index >> 4 & 0b1111;
                                let x = index & 0b1111;

                                chunk.set_block_state(
                                    x,
                                    (y + chunk_y_offset + y_raise) as usize,
                                    z,
                                    state,
                                );
                            }
                        }
                        Ok(())
                    },
                )?;

                // Biome palette
                let mut nbt_biomes: Compound = take_assume(&mut nbt_section, "biomes")?;
                let parsed_biome_palette: Vec<BiomeId> =
                    if let Some(Value::List(List::String(biome_names))) =
                        nbt_biomes.remove("palette")
                    {
                        let mut biomes: Vec<BiomeId> = Vec::with_capacity(biome_names.len());
                        for biome in biome_names {
                            let biome_identity = Ident::new(biome)?;
                            if let Some(biome) = world.biomes.get(&biome_identity) {
                                biomes.push(*biome);
                            } else {
                                return Err(Error::DataFormatError(DataFormatError::UnknownType(
                                    biome_identity,
                                )));
                            }
                        }
                        biomes
                    } else {
                        return Err(Error::NbtFormatError(NbtFormatError::InvalidType(
                            "palette".to_string(),
                        )));
                    };

                palette::parse_palette::<BiomeId, _>(
                    &parsed_biome_palette,
                    take_assume_optional(&mut nbt_biomes, "data"),
                    0,
                    4 * 4 * 4,
                    &mut |data| {
                        match data {
                            DataFormat::All(biome) => {
                                for x in 0..4 {
                                    for y in 0..4isize {
                                        for z in 0..4 {
                                            chunk.set_biome(
                                                x,
                                                (y + (chunk_y_offset / 4) + (y_raise / 4)) as usize,
                                                z,
                                                biome,
                                            );
                                        }
                                    }
                                }
                            }
                            DataFormat::Palette(index, biome) => {
                                let y = (index >> 4 & 0b11) as isize;
                                let z = index >> 2 & 0b11;
                                let x = index & 0b11;

                                let final_y = y + (chunk_y_offset / 4) + (y_raise / 4);
                                chunk.set_biome(x, final_y as usize, z, biome);
                            }
                        }
                        Ok(())
                    },
                )?;
            }

            Ok(chunk)
        } else {
            Err(Error::NbtFormatError(NbtFormatError::InvalidType(
                "sections".to_string(),
            )))
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct AnvilHeader {
    offsets: [ChunkSeekLocation; 1024],
    timestamps: [ChunkTimestamp; 1024],
}

impl AnvilHeader {
    /// Parses the header bytes from the current position
    async fn parse<R: AsyncRead + Unpin>(source: &mut R) -> Result<Self, std::io::Error> {
        let mut offsets = [ChunkSeekLocation::zero(); 1024];
        for offset in &mut offsets {
            let mut buf = [0u8; 4];
            source.read_exact(&mut buf).await?;
            offset.load(buf);
        }
        let mut timestamps = [ChunkTimestamp::zero(); 1024];
        for timestamp in &mut timestamps {
            let mut buf = [0u8; 4];
            source.read_exact(&mut buf).await?;
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
