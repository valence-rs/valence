pub mod batch;
pub mod block;
pub mod chunk;
pub mod index;
mod plugin;

use bevy_ecs::prelude::*;
use bevy_ecs::query::WorldQuery;
use block::BlockRef;
use chunk::LoadedChunk;
pub use index::ChunkIndex;
pub use plugin::*;
use valence_protocol::packets::play::UnloadChunkS2c;
use valence_protocol::{BiomePos, BlockPos, ChunkPos, CompressionThreshold, WritePacket};
use valence_registry::biome::BiomeId;
use valence_registry::dimension_type::DimensionTypeId;
use valence_registry::{BiomeRegistry, DimensionTypeRegistry};
use valence_server_common::Server;

use self::batch::BlockBatch;
use self::block::Block;
use self::chunk::Chunk;
use crate::layer::message::{LayerMessages, MessageScope};
use crate::layer::{ChunkViewIndex, LayerViewers};

#[derive(Component)]
pub struct DimensionLayerBundle {
    pub chunk_index: ChunkIndex,
    pub block_batch: BlockBatch,
    pub dimension_info: DimensionInfo,
    pub chunk_view_index: ChunkViewIndex,
    pub layer_viewers: LayerViewers,
    pub layer_messages: LayerMessages,
}

impl DimensionLayerBundle {
    pub fn new(
        dimension_type: DimensionTypeId,
        dimensions: &DimensionTypeRegistry,
        biomes: &BiomeRegistry,
        server: &Server,
    ) -> Self {
        let dim = &dimensions[dimension_type];

        Self {
            chunk_index: ChunkIndex::new(dim.height),
            block_batch: Default::default(),
            dimension_info: DimensionInfo {
                dimension_type,
                height: dim.height,
                min_y: dim.min_y,
                biome_registry_len: biomes.iter().len() as i32,
                threshold: server.compression_threshold(),
            },
            chunk_view_index: Default::default(),
            layer_viewers: Default::default(),
            layer_messages: LayerMessages::new(server.compression_threshold()),
        }
    }
}

#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct DimensionLayerQuery {
    pub chunk_index: &'static mut ChunkIndex,
    pub block_batch: &'static mut BlockBatch,
    pub dimension_info: &'static DimensionInfo,
    pub chunk_view_index: &'static mut ChunkViewIndex,
    pub layer_viewers: &'static LayerViewers,
    pub layer_messages: &'static mut LayerMessages,
}

macro_rules! immutable_query_methods {
    () => {
        pub fn dimension_type(&self) -> DimensionTypeId {
            self.dimension_info.dimension_type
        }

        pub fn height(&self) -> i32 {
            self.dimension_info.height
        }

        pub fn min_y(&self) -> i32 {
            self.dimension_info.min_y
        }

        pub fn biome(&self, pos: impl Into<BiomePos>) -> Option<BiomeId> {
            todo!()
        }

        pub fn block(&self, pos: impl Into<BlockPos>) -> Option<BlockRef> {
            todo!()
        }

        pub fn chunk(&self, pos: impl Into<ChunkPos>) -> Option<&LoadedChunk> {
            self.chunk_index.get(pos)
        }
    };
}

impl DimensionLayerQueryItem<'_> {
    immutable_query_methods!();

    pub fn set_biome(&mut self, pos: impl Into<BiomePos>, biome: BiomeId) -> Option<BiomeId> {
        todo!()
    }

    pub fn set_block(&mut self, pos: impl Into<BlockPos>, block: impl Into<Block>) {
        todo!()
    }

    pub fn chunk_mut(&mut self, pos: impl Into<ChunkPos>) -> Option<&mut LoadedChunk> {
        self.chunk_index.get_mut(pos)
    }

    pub fn insert_chunk(&mut self, pos: impl Into<ChunkPos>, chunk: Chunk) -> Option<Chunk> {
        match self.chunk_entry(pos) {
            Entry::Occupied(mut entry) => Some(entry.insert(chunk)),
            Entry::Vacant(entry) => {
                entry.insert(chunk);
                None
            }
        }
    }

    pub fn remove_chunk(&mut self, pos: impl Into<ChunkPos>) -> Option<Chunk> {
        match self.chunk_entry(pos) {
            Entry::Occupied(entry) => Some(entry.remove()),
            Entry::Vacant(_) => None,
        }
    }

    pub fn chunk_entry(&mut self, pos: impl Into<ChunkPos>) -> Entry {
        match self.chunk_index.entry(pos) {
            index::Entry::Occupied(entry) => Entry::Occupied(OccupiedEntry {
                chunk_view_index: &*self.chunk_view_index,
                layer_messages: self.layer_messages.reborrow(),
                info: &self.dimension_info,
                entry,
            }),
            index::Entry::Vacant(entry) => Entry::Vacant(VacantEntry {
                chunk_view_index: &*self.chunk_view_index,
                layer_messages: self.layer_messages.reborrow(),
                info: &self.dimension_info,
                entry,
            }),
        }
    }
}

impl DimensionLayerQueryReadOnlyItem<'_> {
    immutable_query_methods!();
}

#[derive(Component, Clone, Debug)]
pub struct DimensionInfo {
    dimension_type: DimensionTypeId,
    height: i32,
    min_y: i32,
    biome_registry_len: i32,
    threshold: CompressionThreshold,
}

impl DimensionInfo {
    pub fn dimension_type(&self) -> DimensionTypeId {
        self.dimension_type
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn min_y(&self) -> i32 {
        self.height
    }
}

#[derive(Debug)]
pub enum Entry<'a> {
    Occupied(OccupiedEntry<'a>),
    Vacant(VacantEntry<'a>),
}

impl<'a> Entry<'a> {
    pub fn or_default(self) -> &'a mut LoadedChunk {
        match self {
            Entry::Occupied(oe) => oe.into_mut(),
            Entry::Vacant(ve) => ve.insert(Chunk::new()),
        }
    }
}

#[derive(Debug)]
pub struct OccupiedEntry<'a> {
    chunk_view_index: &'a ChunkViewIndex,
    layer_messages: Mut<'a, LayerMessages>,
    info: &'a DimensionInfo,
    entry: index::OccupiedEntry<'a>,
}

impl<'a> OccupiedEntry<'a> {
    pub fn get(&self) -> &LoadedChunk {
        self.entry.get()
    }

    pub fn get_mut(&mut self) -> &mut LoadedChunk {
        self.entry.get_mut()
    }

    pub fn insert(&mut self, chunk: Chunk) -> Chunk {
        let pos = *self.key();

        let viewer_count = self.entry.get().viewer_count;
        let res = self.entry.insert(chunk);

        if viewer_count > 0 {
            let loaded = self.entry.get_mut();

            let w = self
                .layer_messages
                .packet_writer(MessageScope::ChunkView { pos });

            loaded.write_chunk_init_packet(w, pos, self.info);
            loaded.viewer_count = viewer_count;
        }

        res
    }

    pub fn into_mut(self) -> &'a mut LoadedChunk {
        self.entry.into_mut()
    }

    pub fn key(&self) -> &ChunkPos {
        self.entry.key()
    }

    pub fn remove(self) -> Chunk {
        self.remove_entry().1
    }

    pub fn remove_entry(mut self) -> (ChunkPos, Chunk) {
        if self.get().viewer_count > 0 {
            self.layer_messages
                .write_packet(&UnloadChunkS2c { pos: *self.key() });
        }

        self.entry.remove_entry()
    }
}

#[derive(Debug)]
pub struct VacantEntry<'a> {
    chunk_view_index: &'a ChunkViewIndex,
    layer_messages: Mut<'a, LayerMessages>,
    info: &'a DimensionInfo,
    entry: index::VacantEntry<'a>,
}

impl<'a> VacantEntry<'a> {
    pub fn insert(mut self, chunk: Chunk) -> &'a mut LoadedChunk {
        let pos = *self.key();

        let viewer_count = self.chunk_view_index.get(pos).len();

        let loaded = self.entry.insert(chunk);

        if viewer_count > 0 {
            let w = self
                .layer_messages
                .packet_writer(MessageScope::ChunkView { pos });

            loaded.write_chunk_init_packet(w, pos, self.info);
            loaded.viewer_count = viewer_count as u32;
        }

        loaded
    }

    pub fn into_key(self) -> ChunkPos {
        self.entry.into_key()
    }

    pub fn key(&self) -> &ChunkPos {
        self.entry.key()
    }
}
