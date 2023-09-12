pub use bevy_ecs::prelude::*;
use rustc_hash::FxHashMap;
use valence_protocol::ChunkPos;
use valence_registry::dimension_type::DimensionTypeId;
use valence_registry::DimensionTypeRegistry;
use valence_server_common::Server;

use super::chunk::{Chunk, LoadedChunk};
use super::DimensionInfo;

/// The mapping of chunk positions to [`LoadedChunk`]s in a dimension layer.
///
/// **NOTE**: Modifying the chunk index directly does not send packets to
/// clients and may lead to desync.
#[derive(Component, Debug)]
pub struct ChunkIndex {
    map: FxHashMap<ChunkPos, LoadedChunk>,
    pub(super) info: DimensionInfo,
}

impl ChunkIndex {
    pub fn new(
        dimension_type: DimensionTypeId,
        dimensions: &DimensionTypeRegistry,
        server: &Server,
    ) -> Self {
        let dim = dimensions[dimension_type];

        Self {
            map: Default::default(),
            info: DimensionInfo {
                dimension_type,
                height: dim.height,
                min_y: dim.min_y,
                biome_registry_len: dimensions.len() as i32,
                threshold: server.compression_threshold(),
            },
        }
    }

    pub(super) fn info(&self) -> &DimensionInfo {
        &self.info
    }

    pub fn dimension_type(&self) -> DimensionTypeId {
        self.info.dimension_type
    }

    pub fn height(&self) -> i32 {
        self.info.height
    }

    pub fn min_y(&self) -> i32 {
        self.info.min_y
    }

    pub fn get(&self, pos: impl Into<ChunkPos>) -> Option<&LoadedChunk> {
        self.map.get(&pos.into())
    }

    pub fn get_mut(&mut self, pos: impl Into<ChunkPos>) -> Option<&mut LoadedChunk> {
        self.map.get_mut(&pos.into())
    }

    pub fn insert(&mut self, pos: impl Into<ChunkPos>, chunk: Chunk) -> Option<Chunk> {
        match self.entry(pos.into()) {
            Entry::Occupied(mut o) => Some(o.insert(chunk)),
            Entry::Vacant(mut v) => {
                v.insert(chunk);
                None
            }
        }
    }

    pub fn remove(&mut self, pos: impl Into<ChunkPos>) -> Option<Chunk> {
        match self.entry(pos.into()) {
            Entry::Occupied(o) => Some(o.remove()),
            Entry::Vacant(_) => None,
        }
    }

    pub fn entry(&mut self, pos: impl Into<ChunkPos>) -> Entry {
        match self.map.entry(pos.into()) {
            std::collections::hash_map::Entry::Occupied(o) => {
                Entry::Occupied(OccupiedEntry { entry: o })
            }
            std::collections::hash_map::Entry::Vacant(v) => Entry::Vacant(VacantEntry {
                entry: v,
                height: self.info.height,
            }),
        }
    }

    // TODO: iter, iter_mut, clear
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
    entry: std::collections::hash_map::OccupiedEntry<'a, ChunkPos, LoadedChunk>,
}

impl<'a> OccupiedEntry<'a> {
    pub fn get(&self) -> &LoadedChunk {
        self.entry.get()
    }

    pub fn get_mut(&mut self) -> &mut LoadedChunk {
        self.entry.get_mut()
    }

    pub fn insert(&mut self, chunk: Chunk) -> Chunk {
        self.entry.get_mut().replace(chunk)
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

    pub fn remove_entry(self) -> (ChunkPos, Chunk) {
        let (pos, chunk) = self.entry.remove_entry();

        (pos, chunk.into_chunk())
    }
}

#[derive(Debug)]
pub struct VacantEntry<'a> {
    entry: std::collections::hash_map::VacantEntry<'a, ChunkPos, LoadedChunk>,
    height: i32,
}

impl<'a> VacantEntry<'a> {
    pub fn insert(self, chunk: Chunk) -> &'a mut LoadedChunk {
        let mut loaded = LoadedChunk::new(self.height);
        loaded.replace(chunk);

        self.entry.insert(loaded)
    }

    pub fn into_key(self) -> ChunkPos {
        *self.entry.key()
    }

    pub fn key(&self) -> &ChunkPos {
        self.entry.key()
    }
}
