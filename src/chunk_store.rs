use std::iter::FusedIterator;

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use rayon::iter::{IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator};

use crate::component::{
    ComponentStore, Components, ComponentsMut, Error, Id, IdData, IdRaw, ZippedComponents,
    ZippedComponentsRaw,
};
use crate::Chunk;

pub struct ChunkStore {
    comps: ComponentStore<ChunkId>,
    chunks: RwLock<Vec<Chunk>>,
}

impl ChunkStore {
    pub(crate) fn new() -> Self {
        Self {
            comps: ComponentStore::new(),
            chunks: RwLock::new(Vec::new()),
        }
    }

    pub fn create(&mut self, height: usize) -> ChunkId {
        assert!(height % 16 == 0, "chunk height must be a multiple of 16");

        let id = self.comps.create_item();
        let chunk = Chunk::new(height / 16);

        let idx = id.0.idx as usize;
        if idx >= self.chunks.get_mut().len() {
            self.chunks.get_mut().push(chunk);
        } else {
            self.chunks.get_mut()[idx] = chunk;
        }

        id
    }

    pub fn delete(&mut self, chunk: ChunkId) -> bool {
        if self.comps.delete_item(chunk) {
            let idx = chunk.0.idx as usize;
            self.chunks.get_mut()[idx].deallocate();
            true
        } else {
            false
        }
    }

    pub fn count(&self) -> usize {
        self.comps.count()
    }

    pub fn is_valid(&self, chunk: ChunkId) -> bool {
        self.comps.is_valid(chunk)
    }

    pub fn get<Z>(&self, z: Z, chunk: ChunkId) -> Option<Z::Item>
    where
        Z: ZippedComponents<Id = ChunkId>,
    {
        self.comps.get(z, chunk)
    }

    pub fn iter<'a, Z>(&'a self, z: Z) -> impl FusedIterator<Item = (ChunkId, Z::Item)> + 'a
    where
        Z: ZippedComponents<Id = ChunkId> + 'a,
    {
        self.comps.iter(z)
    }

    pub fn par_iter<'a, Z>(&'a self, z: Z) -> impl ParallelIterator<Item = (ChunkId, Z::Item)> + 'a
    where
        Z: ZippedComponents<Id = ChunkId> + 'a,
    {
        self.comps.par_iter(z)
    }

    pub fn ids(&self) -> impl FusedIterator<Item = ChunkId> + Clone + '_ {
        self.comps.ids()
    }

    pub fn par_ids(&self) -> impl ParallelIterator<Item = ChunkId> + Clone + '_ {
        self.comps.par_ids()
    }

    pub fn chunks(&self) -> Result<Chunks, Error> {
        Ok(Chunks {
            chunks: self.chunks.try_read().ok_or(Error::NoReadAccess)?,
        })
    }

    pub fn chunks_mut(&self) -> Result<ChunksMut, Error> {
        Ok(ChunksMut {
            chunks: self.chunks.try_write().ok_or(Error::NoWriteAccess)?,
        })
    }

    pub fn register_component<C: 'static + Send + Sync + Default>(&mut self) {
        self.comps.register_component::<C>();
    }

    pub fn unregister_component<C: 'static + Send + Sync + Default>(&mut self) {
        self.comps.unregister_component::<C>()
    }

    pub fn is_registered<C: 'static + Send + Sync + Default>(&self) -> bool {
        self.comps.is_registered::<C>()
    }

    pub fn components<C: 'static + Send + Sync + Default>(
        &self,
    ) -> Result<Components<C, ChunkId>, Error> {
        self.comps.components::<C>()
    }

    pub fn components_mut<C: 'static + Send + Sync + Default>(
        &self,
    ) -> Result<ComponentsMut<C, ChunkId>, Error> {
        self.comps.components_mut::<C>()
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, Debug)]
pub struct ChunkId(pub(crate) IdData);

impl ChunkId {
    /// The value of the default [`ChunkId`] which is always invalid.
    pub const NULL: Self = Self(IdData::NULL);
}

impl IdRaw for ChunkId {
    fn to_data(self) -> IdData {
        self.0
    }

    fn from_data(id: IdData) -> Self {
        Self(id)
    }
}

impl Id for ChunkId {}

pub struct Chunks<'a> {
    chunks: RwLockReadGuard<'a, Vec<Chunk>>,
}

impl<'a, 'b> ZippedComponentsRaw for &'b Chunks<'a> {
    type RawItem = &'b Chunk;
    type RawIter = std::slice::Iter<'b, Chunk>;
    type RawParIter = rayon::slice::Iter<'b, Chunk>;

    fn raw_get(self, idx: usize) -> Self::RawItem {
        &self.chunks[idx]
    }

    fn raw_iter(self) -> Self::RawIter {
        self.chunks.iter()
    }

    fn raw_par_iter(self) -> Self::RawParIter {
        self.chunks.par_iter()
    }
}

impl<'a, 'b> ZippedComponents for &'b Chunks<'a> {
    type Id = ChunkId;
    type Item = &'b Chunk;
}

pub struct ChunksMut<'a> {
    chunks: RwLockWriteGuard<'a, Vec<Chunk>>,
}

impl<'a, 'b> ZippedComponentsRaw for &'b ChunksMut<'a> {
    type RawItem = &'b Chunk;
    type RawIter = std::slice::Iter<'b, Chunk>;
    type RawParIter = rayon::slice::Iter<'b, Chunk>;

    fn raw_get(self, idx: usize) -> Self::RawItem {
        &self.chunks[idx]
    }

    fn raw_iter(self) -> Self::RawIter {
        self.chunks.iter()
    }

    fn raw_par_iter(self) -> Self::RawParIter {
        self.chunks.par_iter()
    }
}

impl<'a, 'b> ZippedComponents for &'b ChunksMut<'a> {
    type Id = ChunkId;
    type Item = &'b Chunk;
}

impl<'a, 'b> ZippedComponentsRaw for &'b mut ChunksMut<'a> {
    type RawItem = &'b mut Chunk;
    type RawIter = std::slice::IterMut<'b, Chunk>;
    type RawParIter = rayon::slice::IterMut<'b, Chunk>;

    fn raw_get(self, idx: usize) -> Self::RawItem {
        &mut self.chunks[idx]
    }

    fn raw_iter(self) -> Self::RawIter {
        self.chunks.iter_mut()
    }

    fn raw_par_iter(self) -> Self::RawParIter {
        self.chunks.par_iter_mut()
    }
}

impl<'a, 'b> ZippedComponents for &'b mut ChunksMut<'a> {
    type Id = ChunkId;
    type Item = &'b mut Chunk;
}
