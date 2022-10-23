mod error;
mod palette;

use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::io::{SeekFrom};
use std::path::{Path, PathBuf};

use async_compression::tokio::bufread::ZlibDecoder;
use async_compression::tokio::write::GzipDecoder;
use byteorder::{BigEndian, ByteOrder};
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncSeek, AsyncSeekExt, AsyncWriteExt};
use tokio::sync::Mutex;
use valence::biome::BiomeId;
use valence::block::{BlockKind, BlockState, PropName, PropValue};
use valence::chunk::{Chunk, ChunkPos, UnloadedChunk};
use valence::ident::Ident;
use valence::nbt::{Compound, List, Value};

use crate::error::Error;
use crate::palette::DataFormat;

#[derive(Debug)]
pub struct AnvilWorld {
    world_root: PathBuf,
    region_files: Mutex<BTreeMap<RegionPos, Option<Region<File>>>>,
}

impl AnvilWorld {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            world_root: directory,
            region_files: Mutex::new(BTreeMap::new()),
        }
    }

    pub async fn load_chunks<I: IntoIterator<Item = ChunkPos>>(
        &self,
        positions: I,
    ) -> Result<Vec<(ChunkPos, Option<UnloadedChunk>)>, Error> {
        let mut map = BTreeMap::<RegionPos, Vec<ChunkPos>>::new();
        for pos in positions.into_iter() {
            let region_pos = RegionPos::from(pos);
            map.entry(region_pos)
                .and_modify(|v| v.push(pos))
                .or_insert(vec![pos]);
        }

        let mut result_vec = Vec::<(ChunkPos, Option<UnloadedChunk>)>::new();
        let mut lock = self.region_files.lock().await;
        for (region_pos, chunk_pos_vec) in map.into_iter() {
            if let Some(region) = lock.entry(region_pos).or_insert({
                let path = region_pos.path(&self.world_root);
                if path.exists() {
                    Some(Region::from_file(File::open(&path).await?).await?)
                } else {
                    None
                }
            }) {
                // A region file exists, and it is loaded.
                result_vec.extend(region.parse_chunks(chunk_pos_vec).await?);
            } else {
                // No region file exists, there is no data to load here.
                result_vec.extend(chunk_pos_vec.into_iter().map(|pos| (pos, None)));
            }
        }

        Ok(result_vec)
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
}

#[derive(Debug)]
pub struct Region<S: AsyncRead + AsyncSeek + Unpin> {
    source: Mutex<S>,
    offset: u64,
    header: AnvilHeader,
}

impl Region<File> {
    /// Convenience method, creates a Region object from the given file.
    pub async fn from_file(source: File) -> Result<Self, std::io::Error> {
        Self::from_seek(Mutex::new(source), 0).await
    }
}

impl<S: AsyncRead + AsyncSeek + Unpin> Region<S> {
    /// Creates a Region object using the incoming stream. The offset defines
    /// the position of the header start.
    pub async fn from_seek(source: Mutex<S>, offset: u64) -> Result<Self, std::io::Error> {
        let mut lock = source.lock().await;
        lock.seek(SeekFrom::Start(offset)).await?;
        let header = AnvilHeader::parse(&mut *lock).await?;
        drop(lock);

        Ok(Self {
            source,
            offset,
            header,
        })
    }

    async fn read_chunk_data(&self, chunk_pos: ChunkPos) -> Result<Option<Vec<u8>>, Error> {
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
            return Err(Error::invalid_chunk_size(compressed_chunk_size));
        }

        let compression = CompressionScheme::from_raw(lock.read_u8().await?)?;
        let uncompressed_buffer = compression
            .read_to_vec(&mut *lock, compressed_chunk_size - 1)
            .await?;
        Ok(Some(uncompressed_buffer))
    }

    pub async fn parse_chunks<I: IntoIterator<Item = ChunkPos>>(
        &self,
        positions: I,
    ) -> Result<Vec<(ChunkPos, Option<UnloadedChunk>)>, Error> {
        let mut results = Vec::<(ChunkPos, Option<UnloadedChunk>)>::new();

        for pos in positions.into_iter() {
            let chunk_data = self.read_chunk_data(pos).await?;
            if let Some(chunk_data) = chunk_data {
                let mut nbt = valence::nbt::from_binary_slice(&mut chunk_data.as_slice())?.0;
                let parsed_chunk = Self::parse_chunk_nbt(&mut nbt)?;
                results.push((pos, Some(parsed_chunk)));
            } else {
                results.push((pos, None));
            }
        }

        Ok(results)
    }

    fn parse_chunk_nbt(nbt: &mut Compound) -> Result<UnloadedChunk, Error> {
        fn take_assume<R>(compound: &mut Compound, key: &'static str) -> Result<R, Error>
            where
                Option<R>: From<Value>,
        {
            match compound.remove(key) {
                None => Err(Error::missing_nbt_value(key)),
                Some(value) => {
                    if let Some(value) = Option::<R>::from(value) {
                        Ok(value)
                    } else {
                        Err(Error::invalid_nbt(key))
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

        // let _chunk_x_pos: i32 = take_assume(nbt, "xPos")?;
        // let _chunk_y_pos: i32 = take_assume(nbt, "yPos")?;
        // let _chunk_z_pos: i32 = take_assume(nbt, "zPos")?;
//
        // let _status: String = take_assume(nbt, "Status")?;
        // let _last_update: i64 = take_assume(nbt, "LastUpdate")?;

        if let Some(Value::List(List::Compound(nbt_sections))) = nbt.remove("sections") {
            let mut y_max = 0i8;
            let mut y_min = 0i8;

            for chunk_nbt in nbt_sections.iter() {
                if let Some(Value::Byte(section_y)) = chunk_nbt.get("Y") {
                    y_max = y_max.max(*section_y);
                    y_min = y_min.min(*section_y);
                } else {
                    return Err(Error::missing_nbt_value("sections/*/Y"));
                }
            }

            // Max should always be equal or higher than 'lower'. Therefore, this is positive.
            let section_height = ((y_max - y_min) as usize * 16) + 16;
            let y_raise = isize::from(-y_min) * 16;

            //Parsing sections
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
                            let block_id = valence::ident::Ident::new(take_assume::<String>(
                                &mut nbt_palette,
                                "Name",
                            )?)?;
                            let block_kind =
                                if let Some(block_kind) = BlockKind::from_str(block_id.path()) {
                                    block_kind
                                } else {
                                    return Err(Error::unknown_type(block_id));
                                };
                            let mut block_state = BlockState::from_kind(block_kind);
                            if let Some(Value::Compound(nbt_palette_properties)) =
                            nbt_palette.remove("Properties")
                            {
                                for (property_name, property_value) in nbt_palette_properties {
                                    if let Value::String(property_value) = property_value {
                                        let property_name = PropName::from_str(&property_name);
                                        let property_value = PropValue::from_str(&property_value);
                                        if let (Some(property_name), Some(property_value)) =
                                        (property_name, property_value)
                                        {
                                            block_state =
                                                block_state.set(property_name, property_value);
                                        } else {
                                            return Err(Error::invalid_nbt(
                                                "sections/*/block_states/Properties/*/property \
                                                 value is not recognized.",
                                            ));
                                        }
                                    } else {
                                        return Err(Error::invalid_nbt(
                                            "sections/*/block_states/Properties/*/property value \
                                             is invalid.",
                                        ));
                                    }
                                }
                            }
                            palette_vec.push(block_state);
                        }
                        palette_vec
                    } else {
                        return Err(Error::invalid_nbt("sections/*/palette"));
                    };

                // Block state palette
                palette::parse_palette::<BlockState, _,>(
                    &parsed_block_state_palette,
                    take_assume_optional(&mut nbt_block_states, "data"),
                    4,
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
                            let _identity_IMPLEMENT_ME = Ident::new(biome)?;

                            //TODO: EXTRACT BIOME IDs
                            //TODO: BiomeId::from_str(identity.path());
                            biomes.push(BiomeId::default());
                        }
                        biomes
                    } else {
                        return Err(Error::invalid_nbt("sections/*/palette."));
                    };

                palette::parse_palette::<BiomeId, _,>(
                    &parsed_biome_palette,
                    take_assume_optional(&mut nbt_biomes, "data"),
                    0,
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
                                chunk.set_biome(
                                    x,
                                    final_y as usize,
                                    z,
                                    biome,
                                );
                            }
                        }
                        Ok(())
                    },
                )?;
            }

            //sections

            Ok(chunk)
        } else {
            return Err(Error::invalid_nbt("sections tag invalid."));
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct AnvilHeader {
    offsets: [ChunkLocation; 1024],
    timestamps: [ChunkTimestamp; 1024],
}

impl AnvilHeader {
    /// Parses the header bytes from the current position
    async fn parse<R: AsyncRead + Unpin>(source: &mut R) -> Result<Self, std::io::Error> {
        let mut offsets = [ChunkLocation::zero(); 1024];
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
    fn offset(&self, x: usize, z: usize) -> &ChunkLocation {
        &self.offsets[(x & 0b11111) + ((z & 0b11111) * 32)]
    }

    #[inline(always)]
    fn timestamp(&self, x: usize, z: usize) -> &ChunkTimestamp {
        &self.timestamps[(x & 0b11111) + ((z & 0b11111) * 32)]
    }
}

/// The location of the chunk inside the region file.
#[derive(Copy, Clone, Debug)]
struct ChunkLocation {
    offset_sectors: u32,
    len_sectors: u8,
}

impl ChunkLocation {
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
struct ChunkTimestamp(u32);

impl Debug for ChunkTimestamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
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
}

#[derive(Debug, Copy, Clone)]
enum CompressionScheme {
    GZip = 1,
    Zlib = 2,
    Raw = 3,
}

impl CompressionScheme {
    fn from_raw(mode: u8) -> Result<Self, Error> {
        match mode {
            1 => Ok(Self::GZip),
            2 => Ok(Self::Zlib),
            3 => Ok(Self::Raw),
            mode => Err(Error::unknown_compression_scheme(mode)),
        }
    }

    async fn read_to_vec<R: AsyncRead + Unpin>(
        self,
        source: &mut R,
        length: usize,
    ) -> Result<Vec<u8>, std::io::Error> {
        let mut raw_data = vec![0u8; length];
        source.read_exact(&mut raw_data).await?;
        match self {
            CompressionScheme::GZip => {
                let mut decoder = GzipDecoder::new(Vec::<u8>::new());
                decoder.write_all(&mut raw_data).await?;
                decoder.shutdown().await?;
                Ok(decoder.into_inner())
            }
            CompressionScheme::Zlib => {
                let mut decoder = ZlibDecoder::new(std::io::Cursor::new(raw_data));
                let mut vec = Vec::<u8>::new();
                decoder.read_to_end(&mut vec).await?;
                Ok(vec)
            }
            CompressionScheme::Raw => {
                Ok(raw_data)
            }
        }
    }
}