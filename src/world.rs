use std::collections::HashMap;
use std::iter::FusedIterator;

use rayon::iter::ParallelIterator;

use crate::chunk::{ChunkId, ChunkPos};
use crate::config::DimensionId;
use crate::slotmap::{Key, SlotMap};
use crate::Id;

pub struct WorldStore {
    sm: SlotMap<World>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct WorldId(Key);

impl Id for WorldId {
    fn idx(self) -> usize {
        self.0.index() as usize
    }
}

impl WorldStore {
    pub(crate) fn new() -> Self {
        Self { sm: SlotMap::new() }
    }

    pub fn count(&self) -> usize {
        self.sm.count()
    }

    pub fn create(&mut self, dim: DimensionId) -> WorldId {
        WorldId(self.sm.insert(World {
            chunks: HashMap::new(),
            dimension: dim,
        }))
    }

    /// Deletes a world from the server. Any [`WorldId`] referring to the
    /// deleted world will be invalidated.
    ///
    /// Note that any entities with positions inside the deleted world will not
    /// be deleted themselves.
    pub fn delete(&mut self, world: WorldId) -> bool {
        self.sm.remove(world.0).is_some()
    }

    pub fn retain(&mut self, mut f: impl FnMut(WorldId, &mut World) -> bool) {
        self.sm.retain(|k, v| f(WorldId(k), v))
    }

    pub fn get(&self, world: WorldId) -> Option<&World> {
        self.sm.get(world.0)
    }

    pub fn get_mut(&mut self, world: WorldId) -> Option<&mut World> {
        self.sm.get_mut(world.0)
    }

    pub fn iter(&self) -> impl FusedIterator<Item = (WorldId, &World)> + Clone + '_ {
        self.sm.iter().map(|(k, v)| (WorldId(k), v))
    }

    pub fn iter_mut(&mut self) -> impl FusedIterator<Item = (WorldId, &mut World)> + '_ {
        self.sm.iter_mut().map(|(k, v)| (WorldId(k), v))
    }

    pub fn par_iter(&self) -> impl ParallelIterator<Item = (WorldId, &World)> + Clone + '_ {
        self.sm.par_iter().map(|(k, v)| (WorldId(k), v))
    }

    pub fn par_iter_mut(&mut self) -> impl ParallelIterator<Item = (WorldId, &mut World)> + '_ {
        self.sm.par_iter_mut().map(|(k, v)| (WorldId(k), v))
    }
}

pub struct World {
    chunks: HashMap<ChunkPos, ChunkId>,
    dimension: DimensionId,
}

impl World {
    pub fn dimension(&self) -> DimensionId {
        self.dimension
    }

    pub fn chunks(&self) -> &HashMap<ChunkPos, ChunkId> {
        &self.chunks
    }

    pub fn chunks_mut(&mut self) -> &mut HashMap<ChunkPos, ChunkId> {
        &mut self.chunks
    }
}
