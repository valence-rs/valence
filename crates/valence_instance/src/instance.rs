//! Contains the [`Instance`] component and methods.

use std::borrow::Cow;
use std::collections::hash_map::{Entry, OccupiedEntry, VacantEntry};

use bevy_ecs::prelude::*;
use glam::{DVec3, Vec3};
use num_integer::div_ceil;
use rustc_hash::FxHashMap;
use valence_biome::BiomeRegistry;
use valence_core::block_pos::BlockPos;
use valence_core::chunk_pos::ChunkPos;
use valence_core::ident::Ident;
use valence_core::protocol::array::LengthPrefixedArray;
use valence_core::protocol::Encode;
use valence_core::sound::{Sound, SoundCategory};
use valence_core::Server;
use valence_dimension::DimensionTypeRegistry;
use valence_nbt::Compound;
use valence_packet::packets::play::particle_s2c::Particle;
use valence_packet::packets::play::{ParticleS2c, PlaySoundS2c};
use valence_packet::protocol::encode::{PacketWriter, WritePacket};
use valence_packet::protocol::Packet;

use crate::chunk::{Block, BlockRef, Chunk, IntoBlock, LoadedChunk, UnloadedChunk, MAX_HEIGHT};

/// An Instance represents a Minecraft world, which consist of [`Chunk`]s.
/// It manages updating clients when chunks change, and caches chunk and entity
/// update packets on a per-chunk basis.
#[derive(Component)]
pub struct Instance {
    pub(super) chunks: FxHashMap<ChunkPos, LoadedChunk>,
    pub(super) info: InstanceInfo,
    /// Packet data to send to all clients in this instance at the end of the
    /// tick.
    pub(super) packet_buf: Vec<u8>,
}

#[doc(hidden)]
pub struct InstanceInfo {
    pub(super) dimension_type_name: Ident<String>,
    pub(super) height: u32,
    pub(super) min_y: i32,
    pub(super) biome_registry_len: usize,
    pub(super) compression_threshold: Option<u32>,
    // We don't have a proper lighting engine yet, so we just fill chunks with full brightness.
    pub(super) sky_light_mask: Box<[u64]>,
    pub(super) sky_light_arrays: Box<[LengthPrefixedArray<u8, 2048>]>,
}

impl Instance {
    #[track_caller]
    pub fn new(
        dimension_type_name: impl Into<Ident<String>>,
        dimensions: &DimensionTypeRegistry,
        biomes: &BiomeRegistry,
        server: &Server,
    ) -> Self {
        let dimension_type_name = dimension_type_name.into();

        let dim = &dimensions[dimension_type_name.as_str_ident()];

        assert!(
            (0..MAX_HEIGHT as i32).contains(&dim.height),
            "invalid dimension height of {}",
            dim.height
        );

        let light_section_count = (dim.height / 16 + 2) as usize;

        let mut sky_light_mask = vec![0; div_ceil(light_section_count, 16)];

        for i in 0..light_section_count {
            sky_light_mask[i / 64] |= 1 << (i % 64);
        }

        Self {
            chunks: Default::default(),
            info: InstanceInfo {
                dimension_type_name,
                height: dim.height as u32,
                min_y: dim.min_y,
                biome_registry_len: biomes.iter().len(),
                compression_threshold: server.compression_threshold(),
                sky_light_mask: sky_light_mask.into(),
                sky_light_arrays: vec![LengthPrefixedArray([0xff; 2048]); light_section_count]
                    .into(),
            },
            packet_buf: vec![],
        }
    }

    pub fn dimension_type_name(&self) -> Ident<&str> {
        self.info.dimension_type_name.as_str_ident()
    }

    /// The height of this instance's dimension.
    pub fn height(&self) -> u32 {
        self.info.height
    }

    /// The `min_y` of this instance's dimension.
    pub fn min_y(&self) -> i32 {
        self.info.min_y
    }

    /// Get a reference to the chunk at the given position, if it is loaded.
    pub fn chunk(&self, pos: impl Into<ChunkPos>) -> Option<&LoadedChunk> {
        self.chunks.get(&pos.into())
    }

    /// Get a mutable reference to the chunk at the given position, if it is
    /// loaded.
    pub fn chunk_mut(&mut self, pos: impl Into<ChunkPos>) -> Option<&mut LoadedChunk> {
        self.chunks.get_mut(&pos.into())
    }

    /// Insert a chunk into the instance at the given position. The preivous
    /// chunk data is returned.
    pub fn insert_chunk(
        &mut self,
        pos: impl Into<ChunkPos>,
        chunk: UnloadedChunk,
    ) -> Option<UnloadedChunk> {
        match self.chunk_entry(pos) {
            ChunkEntry::Occupied(mut oe) => Some(oe.insert(chunk)),
            ChunkEntry::Vacant(ve) => {
                ve.insert(chunk);
                None
            }
        }
    }

    /// Unload the chunk at the given position, if it is loaded. Returns the
    /// chunk if it was loaded.
    pub fn remove_chunk(&mut self, pos: impl Into<ChunkPos>) -> Option<UnloadedChunk> {
        match self.chunk_entry(pos) {
            ChunkEntry::Occupied(oe) => Some(oe.remove()),
            ChunkEntry::Vacant(_) => None,
        }
    }

    /// Unload all chunks in this instance.
    pub fn clear_chunks(&mut self) {
        self.retain_chunks(|_, _| false)
    }

    /// Retain only the chunks for which the given predicate returns `true`.
    pub fn retain_chunks<F>(&mut self, mut f: F)
    where
        F: FnMut(ChunkPos, &mut LoadedChunk) -> bool,
    {
        self.chunks.retain(|pos, chunk| f(*pos, chunk));
    }

    /// Get a [`ChunkEntry`] for the given position.
    pub fn chunk_entry(&mut self, pos: impl Into<ChunkPos>) -> ChunkEntry {
        match self.chunks.entry(pos.into()) {
            Entry::Occupied(oe) => ChunkEntry::Occupied(OccupiedChunkEntry { entry: oe }),
            Entry::Vacant(ve) => ChunkEntry::Vacant(VacantChunkEntry {
                height: self.info.height,
                compression_threshold: self.info.compression_threshold,
                entry: ve,
            }),
        }
    }

    /// Get an iterator over all loaded chunks in the instance. The order of the
    /// chunks is undefined.
    pub fn chunks(&self) -> impl Iterator<Item = (ChunkPos, &LoadedChunk)> + Clone + '_ {
        self.chunks.iter().map(|(pos, chunk)| (*pos, chunk))
    }

    /// Get an iterator over all loaded chunks in the instance, mutably. The
    /// order of the chunks is undefined.
    pub fn chunks_mut(&mut self) -> impl Iterator<Item = (ChunkPos, &mut LoadedChunk)> + '_ {
        self.chunks.iter_mut().map(|(pos, chunk)| (*pos, chunk))
    }

    /// Optimizes the memory usage of the instance.
    pub fn optimize(&mut self) {
        for (_, chunk) in self.chunks_mut() {
            chunk.optimize();
        }

        self.chunks.shrink_to_fit();
        self.packet_buf.shrink_to_fit();
    }

    pub fn block(&self, pos: impl Into<BlockPos>) -> Option<BlockRef> {
        let (chunk, x, y, z) = self.chunk_and_offsets(pos.into())?;
        Some(chunk.block(x, y, z))
    }

    pub fn set_block(&mut self, pos: impl Into<BlockPos>, block: impl IntoBlock) -> Option<Block> {
        let (chunk, x, y, z) = self.chunk_and_offsets_mut(pos.into())?;
        Some(chunk.set_block(x, y, z, block))
    }

    pub fn block_entity_mut(&mut self, pos: impl Into<BlockPos>) -> Option<&mut Compound> {
        let (chunk, x, y, z) = self.chunk_and_offsets_mut(pos.into())?;
        chunk.block_entity_mut(x, y, z)
    }

    #[inline]
    fn chunk_and_offsets(&self, pos: BlockPos) -> Option<(&LoadedChunk, u32, u32, u32)> {
        let Some(y) = pos
            .y
            .checked_sub(self.info.min_y)
            .and_then(|y| y.try_into().ok())
        else {
            return None;
        };

        if y >= self.info.height {
            return None;
        }

        let Some(chunk) = self.chunk(ChunkPos::from_block_pos(pos)) else {
            return None;
        };

        let x = pos.x.rem_euclid(16) as u32;
        let z = pos.z.rem_euclid(16) as u32;

        Some((chunk, x, y, z))
    }

    #[inline]
    fn chunk_and_offsets_mut(
        &mut self,
        pos: BlockPos,
    ) -> Option<(&mut LoadedChunk, u32, u32, u32)> {
        let Some(y) = pos
            .y
            .checked_sub(self.info.min_y)
            .and_then(|y| y.try_into().ok())
        else {
            return None;
        };

        if y >= self.info.height {
            return None;
        }

        let Some(chunk) = self.chunk_mut(ChunkPos::from_block_pos(pos)) else {
            return None;
        };

        let x = pos.x.rem_euclid(16) as u32;
        let z = pos.z.rem_euclid(16) as u32;

        Some((chunk, x, y, z))
    }

    /// Writes a packet to all clients in view of `pos` in this instance. Has no
    /// effect if there is no chunk at `pos`.
    ///
    /// This is more efficient than sending the packet to each client
    /// individually.
    pub fn write_packet_at<P>(&mut self, pkt: &P, pos: impl Into<ChunkPos>)
    where
        P: Packet + Encode,
    {
        if let Some(chunk) = self.chunks.get_mut(&pos.into()) {
            chunk.write_packet(pkt);
        }
    }

    /// Writes arbitrary packet data to all clients in view of `pos` in this
    /// instance. Has no effect if there is no chunk at `pos`.
    ///
    /// The packet data must be properly compressed for the current compression
    /// threshold but never encrypted. Don't use this function unless you know
    /// what you're doing. Consider using [`Self::write_packet`] instead.
    pub fn write_packet_bytes_at(&mut self, bytes: &[u8], pos: impl Into<ChunkPos>) {
        if let Some(chunk) = self.chunks.get_mut(&pos.into()) {
            chunk.write_packet_bytes(bytes);
        }
    }

    /// An immutable view into this instance's packet buffer.
    #[doc(hidden)]
    pub fn packet_buf(&self) -> &[u8] {
        &self.packet_buf
    }

    #[doc(hidden)]
    pub fn info(&self) -> &InstanceInfo {
        &self.info
    }

    // TODO: move to `valence_particle`.
    /// Puts a particle effect at the given position in the world. The particle
    /// effect is visible to all players in the instance with the
    /// appropriate chunk in view.
    pub fn play_particle(
        &mut self,
        particle: &Particle,
        long_distance: bool,
        position: impl Into<DVec3>,
        offset: impl Into<Vec3>,
        max_speed: f32,
        count: i32,
    ) {
        let position = position.into();

        self.write_packet_at(
            &ParticleS2c {
                particle: Cow::Borrowed(particle),
                long_distance,
                position,
                offset: offset.into(),
                max_speed,
                count,
            },
            ChunkPos::from_dvec3(position),
        );
    }

    // TODO: move to `valence_sound`.
    /// Plays a sound effect at the given position in the world. The sound
    /// effect is audible to all players in the instance with the
    /// appropriate chunk in view.
    pub fn play_sound(
        &mut self,
        sound: Sound,
        category: SoundCategory,
        position: impl Into<DVec3>,
        volume: f32,
        pitch: f32,
    ) {
        let position = position.into();

        self.write_packet_at(
            &PlaySoundS2c {
                id: sound.to_id(),
                category,
                position: (position * 8.0).as_ivec3(),
                volume,
                pitch,
                seed: rand::random(),
            },
            ChunkPos::from_dvec3(position),
        );
    }
}

/// Writing packets to the instance writes to the instance's global packet
/// buffer. All clients in the instance will receive the packet at the end of
/// the tick.
///
/// This is generally more efficient than sending the packet to each client
/// individually.
impl WritePacket for Instance {
    fn write_packet_fallible<P>(&mut self, packet: &P) -> anyhow::Result<()>
    where
        P: Packet + Encode,
    {
        PacketWriter::new(&mut self.packet_buf, self.info.compression_threshold)
            .write_packet_fallible(packet)
    }

    fn write_packet_bytes(&mut self, bytes: &[u8]) {
        self.packet_buf.extend_from_slice(bytes)
    }
}

#[derive(Debug)]
pub enum ChunkEntry<'a> {
    Occupied(OccupiedChunkEntry<'a>),
    Vacant(VacantChunkEntry<'a>),
}

impl<'a> ChunkEntry<'a> {
    pub fn or_default(self) -> &'a mut LoadedChunk {
        match self {
            ChunkEntry::Occupied(oe) => oe.into_mut(),
            ChunkEntry::Vacant(ve) => ve.insert(UnloadedChunk::new()),
        }
    }
}

#[derive(Debug)]
pub struct OccupiedChunkEntry<'a> {
    entry: OccupiedEntry<'a, ChunkPos, LoadedChunk>,
}

impl<'a> OccupiedChunkEntry<'a> {
    pub fn get(&self) -> &LoadedChunk {
        self.entry.get()
    }

    pub fn get_mut(&mut self) -> &mut LoadedChunk {
        self.entry.get_mut()
    }

    pub fn insert(&mut self, chunk: UnloadedChunk) -> UnloadedChunk {
        self.entry.get_mut().insert(chunk)
    }

    pub fn into_mut(self) -> &'a mut LoadedChunk {
        self.entry.into_mut()
    }

    pub fn key(&self) -> &ChunkPos {
        self.entry.key()
    }

    pub fn remove(mut self) -> UnloadedChunk {
        self.entry.get_mut().remove()
    }

    pub fn remove_entry(mut self) -> (ChunkPos, UnloadedChunk) {
        let pos = *self.entry.key();
        let chunk = self.entry.get_mut().remove();

        (pos, chunk)
    }
}

#[derive(Debug)]
pub struct VacantChunkEntry<'a> {
    height: u32,
    compression_threshold: Option<u32>,
    entry: VacantEntry<'a, ChunkPos, LoadedChunk>,
}

impl<'a> VacantChunkEntry<'a> {
    pub fn insert(self, chunk: UnloadedChunk) -> &'a mut LoadedChunk {
        let mut loaded = LoadedChunk::new(self.height, self.compression_threshold);
        loaded.insert(chunk);

        self.entry.insert(loaded)
    }

    pub fn into_key(self) -> ChunkPos {
        *self.entry.key()
    }

    pub fn key(&self) -> &ChunkPos {
        self.entry.key()
    }
}
