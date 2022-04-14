use std::collections::HashMap;
use std::iter::FusedIterator;

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use rayon::iter::{IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator};

use crate::chunk::ChunkPos;
use crate::chunk_store::ChunkId;
use crate::component::{
    ComponentStore, Components, ComponentsMut, Error, Id, IdData, IdRaw, ZippedComponents,
    ZippedComponentsRaw,
};
use crate::config::DimensionId;
use crate::SharedServer;

pub struct WorldStore {
    comps: ComponentStore<WorldId>,
    worlds: RwLock<Vec<World>>,
    shared: SharedServer,
}

impl WorldStore {
    pub(crate) fn new(shared: SharedServer) -> Self {
        Self {
            comps: ComponentStore::new(),
            worlds: RwLock::new(Vec::new()),
            shared,
        }
    }

    pub fn create(&mut self, dim: DimensionId) -> WorldId {
        let height = self.shared.dimension(dim).height;

        let id = self.comps.create_item();
        let world = World {
            chunks: HashMap::new(),
            dimension: dim,
        };

        let idx = id.0.idx as usize;
        if idx >= self.worlds.get_mut().len() {
            self.worlds.get_mut().push(world);
        } else {
            self.worlds.get_mut()[idx] = world;
        }

        id
    }

    /// Deletes a world from the server. Any [`WorldId`] referring to the
    /// deleted world will be invalidated.
    ///
    /// Note that any entities with positions inside the deleted world will not
    /// be deleted themselves. These entities should be deleted or moved
    /// elsewhere, preferably before calling this function.
    pub fn delete(&mut self, world: WorldId) -> bool {
        if self.comps.delete_item(world) {
            let idx = world.0.idx as usize;
            self.worlds.get_mut()[idx].chunks = HashMap::new();
            true
        } else {
            false
        }
    }

    pub fn count(&self) -> usize {
        self.comps.count()
    }

    pub fn get<Z>(&self, z: Z, world: WorldId) -> Option<Z::Item>
    where
        Z: ZippedComponents<Id = WorldId>,
    {
        self.comps.get(z, world)
    }

    pub fn iter<'a, Z>(&'a self, z: Z) -> impl FusedIterator<Item = (WorldId, Z::Item)> + 'a
    where
        Z: ZippedComponents<Id = WorldId> + 'a,
    {
        self.comps.iter(z)
    }

    pub fn par_iter<'a, Z>(&'a self, z: Z) -> impl ParallelIterator<Item = (WorldId, Z::Item)> + 'a
    where
        Z: ZippedComponents<Id = WorldId> + 'a,
    {
        self.comps.par_iter(z)
    }

    pub fn ids(&self) -> impl FusedIterator<Item = WorldId> + Clone + '_ {
        self.comps.ids()
    }

    pub fn par_ids(&self) -> impl ParallelIterator<Item = WorldId> + Clone + '_ {
        self.comps.par_ids()
    }

    pub fn worlds(&self) -> Result<Worlds, Error> {
        Ok(Worlds {
            worlds: self.worlds.try_read().ok_or(Error::NoReadAccess)?,
        })
    }

    pub fn worlds_mut(&self) -> Result<WorldsMut, Error> {
        Ok(WorldsMut {
            worlds: self.worlds.try_write().ok_or(Error::NoWriteAccess)?,
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
    ) -> Result<Components<C, WorldId>, Error> {
        self.comps.components::<C>()
    }

    pub fn components_mut<C: 'static + Send + Sync + Default>(
        &self,
    ) -> Result<ComponentsMut<C, WorldId>, Error> {
        self.comps.components_mut::<C>()
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, Debug)]
pub struct WorldId(pub(crate) IdData);

impl WorldId {
    /// The value of the default [`WorldId`] which always refers to an invalid
    /// world.
    pub const NULL: Self = Self(IdData::NULL);
}

impl IdRaw for WorldId {
    fn to_data(self) -> IdData {
        self.0
    }

    fn from_data(id: IdData) -> Self {
        Self(id)
    }
}

impl Id for WorldId {}

pub struct Worlds<'a> {
    worlds: RwLockReadGuard<'a, Vec<World>>,
}

impl<'a, 'b> ZippedComponentsRaw for &'b Worlds<'a> {
    type RawItem = &'b World;
    type RawIter = std::slice::Iter<'b, World>;
    type RawParIter = rayon::slice::Iter<'b, World>;

    fn raw_get(self, idx: usize) -> Self::RawItem {
        &self.worlds[idx]
    }

    fn raw_iter(self) -> Self::RawIter {
        self.worlds.iter()
    }

    fn raw_par_iter(self) -> Self::RawParIter {
        self.worlds.par_iter()
    }
}

impl<'a, 'b> ZippedComponents for &'b Worlds<'a> {
    type Id = WorldId;
    type Item = &'b World;
}

pub struct WorldsMut<'a> {
    worlds: RwLockWriteGuard<'a, Vec<World>>,
}

impl<'a, 'b> ZippedComponentsRaw for &'b WorldsMut<'a> {
    type RawItem = &'b World;
    type RawIter = std::slice::Iter<'b, World>;
    type RawParIter = rayon::slice::Iter<'b, World>;

    fn raw_get(self, idx: usize) -> Self::RawItem {
        &self.worlds[idx]
    }

    fn raw_iter(self) -> Self::RawIter {
        self.worlds.iter()
    }

    fn raw_par_iter(self) -> Self::RawParIter {
        self.worlds.par_iter()
    }
}

impl<'a, 'b> ZippedComponents for &'b WorldsMut<'a> {
    type Id = WorldId;
    type Item = &'b World;
}

impl<'a, 'b> ZippedComponentsRaw for &'b mut WorldsMut<'a> {
    type RawItem = &'b mut World;
    type RawIter = std::slice::IterMut<'b, World>;
    type RawParIter = rayon::slice::IterMut<'b, World>;

    fn raw_get(self, idx: usize) -> Self::RawItem {
        &mut self.worlds[idx]
    }

    fn raw_iter(self) -> Self::RawIter {
        self.worlds.iter_mut()
    }

    fn raw_par_iter(self) -> Self::RawParIter {
        self.worlds.par_iter_mut()
    }
}

impl<'a, 'b> ZippedComponents for &'b mut WorldsMut<'a> {
    type Id = WorldId;
    type Item = &'b mut World;
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
