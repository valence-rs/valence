use std::collections::hash_map::{Entry, OccupiedEntry};

use super::*;

#[derive(Debug)]
pub enum ChunkEntry<'a> {
    Occupied(OccupiedChunkEntry<'a>),
    Vacant(VacantChunkEntry<'a>),
}

impl<'a> ChunkEntry<'a> {
    pub(super) fn new(section_count: usize, entry: Entry<'a, ChunkPos, PartitionCell>) -> Self {
        match entry {
            Entry::Occupied(oe) => {
                if oe.get().chunk.is_some() {
                    ChunkEntry::Occupied(OccupiedChunkEntry {
                        section_count,
                        entry: oe,
                    })
                } else {
                    ChunkEntry::Vacant(VacantChunkEntry {
                        section_count,
                        entry: Entry::Occupied(oe),
                    })
                }
            }
            Entry::Vacant(ve) => ChunkEntry::Vacant(VacantChunkEntry {
                section_count,
                entry: Entry::Vacant(ve),
            }),
        }
    }

    pub fn or_default(self) -> &'a mut Chunk<true> {
        match self {
            ChunkEntry::Occupied(oe) => oe.into_mut(),
            ChunkEntry::Vacant(ve) => ve.insert(Chunk::default()),
        }
    }
}

#[derive(Debug)]
pub struct OccupiedChunkEntry<'a> {
    section_count: usize,
    entry: OccupiedEntry<'a, ChunkPos, PartitionCell>,
}

impl<'a> OccupiedChunkEntry<'a> {
    pub fn get(&self) -> &Chunk<true> {
        self.entry.get().chunk.as_ref().unwrap()
    }

    pub fn get_mut(&mut self) -> &mut Chunk<true> {
        self.entry.get_mut().chunk.as_mut().unwrap()
    }

    pub fn insert(&mut self, mut chunk: Chunk) -> Chunk {
        chunk.resize(self.section_count);

        self.entry
            .get_mut()
            .chunk
            .replace(chunk.into_loaded())
            .unwrap()
            .into_unloaded()
    }

    pub fn into_mut(self) -> &'a mut Chunk<true> {
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
    section_count: usize,
    entry: Entry<'a, ChunkPos, PartitionCell>,
}

impl<'a> VacantChunkEntry<'a> {
    pub fn insert(self, mut chunk: Chunk) -> &'a mut Chunk<true> {
        chunk.resize(self.section_count);

        let cell = self.entry.or_insert_with(|| PartitionCell {
            chunk: None,
            chunk_removed: false,
            entities: BTreeSet::new(),
            incoming: vec![],
            outgoing: vec![],
            packet_buf: vec![],
            layers_packet_buf: [0; 64].map(|_| vec![]),
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
