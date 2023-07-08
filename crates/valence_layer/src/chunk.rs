//! Contains the [`Instance`] component and methods.

mod chunk;
pub mod loaded;
mod paletted_container;
pub mod unloaded;

use std::collections::hash_map::{Entry, OccupiedEntry, VacantEntry};

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use num_integer::div_ceil;
use rustc_hash::FxHashMap;
use valence_biome::BiomeRegistry;
use valence_core::block_pos::BlockPos;
use valence_core::chunk_pos::ChunkPos;
use valence_core::ident::Ident;
use valence_core::protocol::array::LengthPrefixedArray;
use valence_core::Server;
use valence_dimension::DimensionTypeRegistry;
use valence_nbt::Compound;

use crate::bvh::GetChunkPos;
use crate::chunk::chunk::MAX_HEIGHT;
use crate::message::Messages;
use crate::{Layer, UpdateLayersPreClient};

pub use self::chunk::*;
pub use self::loaded::LoadedChunk;
pub use self::unloaded::UnloadedChunk;

#[derive(Component, Debug)]
pub struct ChunkLayer {
    messages: ChunkLayerMessages,
    chunks: FxHashMap<ChunkPos, LoadedChunk>,
    info: ChunkLayerInfo,
}

#[doc(hidden)]
#[derive(Debug)]
pub struct ChunkLayerInfo {
    dimension_type_name: Ident<String>,
    height: u32,
    min_y: i32,
    biome_registry_len: usize,
    compression_threshold: Option<u32>,
    // We don't have a proper lighting engine yet, so we just fill chunks with full brightness.
    sky_light_mask: Box<[u64]>,
    sky_light_arrays: Box<[LengthPrefixedArray<u8, 2048>]>,
}

type ChunkLayerMessages = Messages<GlobalMsg, LocalMsg>;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum GlobalMsg {
    /// Send packet data to all clients viewing the layer.
    Packet,
    /// Send packet data to all clients viewing layer, except the client identified by `except`.
    PacketExcept,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum LocalMsg {
    /// Send packet data to all clients viewing the layer in view of `pos`.
    PacketAt {
        pos: ChunkPos,
    },
    ChangeBiome {
        pos: ChunkPos,
    },
}

impl GetChunkPos for LocalMsg {
    fn chunk_pos(&self) -> ChunkPos {
        match *self {
            LocalMsg::PacketAt { pos } => pos,
            LocalMsg::ChangeBiome { pos } => pos,
        }
    }
}

impl Layer for ChunkLayer {
    type Global = GlobalMsg;

    type Local = LocalMsg;

    fn send_global(&mut self, msg: Self::Global, f: impl FnOnce(&mut Vec<u8>)) {
        self.messages.send_global(msg, f)
    }

    fn send_local(&mut self, msg: Self::Local, f: impl FnOnce(&mut Vec<u8>)) {
        self.messages.send_local(msg, f)
    }

    fn compression_threshold(&self) -> Option<u32> {
        self.info.compression_threshold
    }
}

impl ChunkLayer {
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
            messages: Messages::new(),
            chunks: Default::default(),
            info: ChunkLayerInfo {
                dimension_type_name,
                height: dim.height as u32,
                min_y: dim.min_y,
                biome_registry_len: biomes.iter().len(),
                compression_threshold: server.compression_threshold(),
                sky_light_mask: sky_light_mask.into(),
                sky_light_arrays: vec![LengthPrefixedArray([0xff; 2048]); light_section_count]
                    .into(),
            },
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
        for (pos, chunk) in &mut self.chunks {
            if !f(*pos, chunk) {
                chunk.remove();
            }
        }
    }

    /// Get a [`ChunkEntry`] for the given position.
    pub fn chunk_entry(&mut self, pos: impl Into<ChunkPos>) -> ChunkEntry {
        match self.chunks.entry(pos.into()) {
            Entry::Occupied(oe) => ChunkEntry::Occupied(OccupiedChunkEntry { entry: oe }),
            Entry::Vacant(ve) => ChunkEntry::Vacant(VacantChunkEntry {
                height: self.info.height,
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
            chunk.shrink_to_fit();
        }

        self.chunks.shrink_to_fit();
        self.messages.shrink_to_fit();
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
        let Some(y) = pos.y.checked_sub(self.info.min_y).and_then(|y| y.try_into().ok()) else {
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
        let Some(y) = pos.y.checked_sub(self.info.min_y).and_then(|y| y.try_into().ok()) else {
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

    #[doc(hidden)]
    pub fn info(&self) -> &ChunkLayerInfo {
        &self.info
    }

    #[doc(hidden)]
    pub fn messages(&self) -> &ChunkLayerMessages {
        &self.messages
    }

    /*
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
    }*/
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
    entry: VacantEntry<'a, ChunkPos, LoadedChunk>,
}

impl<'a> VacantChunkEntry<'a> {
    pub fn insert(self, chunk: UnloadedChunk) -> &'a mut LoadedChunk {
        let mut loaded = LoadedChunk::new(self.height);
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

pub(super) fn build(app: &mut App) {
    app.add_systems(
        PostUpdate,
        (
            update_chunks_pre_client.in_set(UpdateLayersPreClient),
            update_chunks_post_client.in_set(UpdateLayersPreClient),
        ),
    );
}

fn update_chunks_pre_client(mut layers: Query<&mut ChunkLayer>) {
    for layer in &mut layers {
        let layer = layer.into_inner();

        for (&pos, chunk) in &mut layer.chunks {
            chunk.update_pre_client(pos, &layer.info, &mut layer.messages);
        }

        layer.messages.ready();
    }
}

fn update_chunks_post_client(mut layers: Query<&mut ChunkLayer>) {
    for mut layer in &mut layers {
        layer
            .chunks
            .retain(|&pos, chunk| chunk.update_post_client(pos));

        layer.messages.unready();
    }
}
