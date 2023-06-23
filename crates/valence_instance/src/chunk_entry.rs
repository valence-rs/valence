use std::collections::hash_map::{Entry, OccupiedEntry, VacantEntry};

use crate::chunk::UnloadedChunk;

use super::*;

#[derive(Debug)]
pub enum ChunkEntry<'a> {
    Occupied(OccupiedChunkEntry<'a>),
    Vacant(VacantChunkEntry<'a>),
}

impl<'a> ChunkEntry<'a> {
    pub(super) fn new(height: u32, entry: Entry<'a, ChunkPos, LoadedChunk>) -> Self {
        match entry {
            Entry::Occupied(oe) => ChunkEntry::Occupied(OccupiedChunkEntry { height, entry: oe }),
            Entry::Vacant(ve) => ChunkEntry::Vacant(VacantChunkEntry { height, entry: ve }),
        }
    }

    pub fn or_default(self) -> &'a mut LoadedChunk {
        match self {
            ChunkEntry::Occupied(oe) => oe.into_mut(),
            ChunkEntry::Vacant(ve) => ve.insert(Chunk::default()),
        }
    }
}

#[derive(Debug)]
pub struct OccupiedChunkEntry<'a> {
    height: u32,
    entry: OccupiedEntry<'a, ChunkPos, LoadedChunk>,
}

impl<'a> OccupiedChunkEntry<'a> {
    pub fn get(&self) -> &LoadedChunk {
        self.entry.get()
    }

    pub fn get_mut(&mut self) -> &mut LoadedChunk {
        self.entry.get_mut()
    }

    pub fn insert(&mut self, mut chunk: UnloadedChunk) -> Chunk {
        chunk.set_height(self.height);

        self.entry
            .get_mut()
            .chunk
            .replace(chunk.into_loaded())
            .unwrap()
            .into_unloaded()
    }

    pub fn into_mut(self) -> &'a mut LoadedChunk {
        self.entry.into_mut().chunk.as_mut().unwrap()
    }

    pub fn key(&self) -> &ChunkPos {
        self.entry.key()
    }

    pub fn remove(self) -> Chunk {
        let cell = self.entry.into_mut();
        cell.chunk_removed = true;
        cell.chunk.take().unwrap().into_unloaded()
    }

    pub fn remove_entry(self) -> (ChunkPos, Chunk) {
        let pos = *self.entry.key();
        let cell = self.entry.into_mut();
        cell.chunk_removed = true;
        (pos, cell.chunk.take().unwrap().into_unloaded())
    }
}

#[derive(Debug)]
pub struct VacantChunkEntry<'a> {
    height: u32,
    entry: VacantEntry<'a, ChunkPos, LoadedChunk>,
}

impl<'a> VacantChunkEntry<'a> {
    pub fn insert(self, mut chunk: Chunk) -> &'a mut LoadedChunk {
        chunk.resize(self.section_count);

        let cell = self.entry.or_insert_with(|| PartitionCell {
            chunk: None,
            chunk_removed: false,
            entities: BTreeSet::new(),
            incoming: vec![],
            outgoing: vec![],
            packet_buf: vec![],
        });

        debug_assert!(cell.chunk.is_none());
        cell.chunk.insert(chunk.into_loaded())
    }

    pub fn into_key(self) -> ChunkPos {
        *self.entry.key()
    }

    pub fn key(&self) -> &ChunkPos {
        self.entry.key()
    }
}
