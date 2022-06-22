use std::iter::FusedIterator;
use std::ops::Deref;

use rayon::iter::ParallelIterator;

use crate::slotmap::{Key, SlotMap};
use crate::{
    Chunks, ChunksMut, Clients, ClientsMut, DimensionId, Entities, EntitiesMut, Server,
    SpatialIndex, SpatialIndexMut,
};

pub struct Worlds {
    sm: SlotMap<World>,
    server: Server,
}

pub struct WorldsMut<'a>(&'a mut Worlds);

impl<'a> Deref for WorldsMut<'a> {
    type Target = Worlds;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct WorldId(Key);

impl Worlds {
    pub(crate) fn new(server: Server) -> Self {
        Self {
            sm: SlotMap::new(),
            server,
        }
    }

    pub fn count(&self) -> usize {
        self.sm.len()
    }

    pub fn get(&self, world: WorldId) -> Option<WorldRef> {
        self.sm.get(world.0).map(WorldRef::new)
    }

    pub fn iter(&self) -> impl FusedIterator<Item = (WorldId, WorldRef)> + Clone + '_ {
        self.sm.iter().map(|(k, v)| (WorldId(k), WorldRef::new(v)))
    }
}

impl<'a> WorldsMut<'a> {
    pub(crate) fn new(worlds: &'a mut Worlds) -> Self {
        Self(worlds)
    }

    pub fn reborrow(&mut self) -> WorldsMut {
        WorldsMut(self.0)
    }

    pub fn create(&mut self, dim: DimensionId) -> (WorldId, WorldMut) {
        let (id, world) = self.0.sm.insert(World {
            clients: Clients::new(),
            entities: Entities::new(),
            spatial_index: SpatialIndex::new(),
            chunks: Chunks::new(self.server.clone(), dim),
            meta: WorldMeta {
                dimension: dim,
                is_flat: false,
            },
        });

        (WorldId(id), WorldMut::new(world))
    }

    /// Deletes a world from the server. Any [`WorldId`] referring to the
    /// deleted world will be invalidated.
    ///
    /// Note that any entities with positions inside the deleted world will not
    /// be deleted themselves.
    pub fn delete(&mut self, world: WorldId) -> bool {
        self.0.sm.remove(world.0).is_some()
    }

    pub fn retain(&mut self, mut f: impl FnMut(WorldId, WorldMut) -> bool) {
        self.0.sm.retain(|k, v| f(WorldId(k), WorldMut::new(v)))
    }

    pub fn get_mut(&mut self, world: WorldId) -> Option<WorldMut> {
        self.0.sm.get_mut(world.0).map(WorldMut::new)
    }

    pub fn iter_mut(&mut self) -> impl FusedIterator<Item = (WorldId, WorldMut)> + '_ {
        self.0
            .sm
            .iter_mut()
            .map(|(k, v)| (WorldId(k), WorldMut::new(v)))
    }

    pub fn par_iter(&self) -> impl ParallelIterator<Item = (WorldId, WorldRef)> + Clone + '_ {
        self.0
            .sm
            .par_iter()
            .map(|(k, v)| (WorldId(k), WorldRef::new(v)))
    }

    pub fn par_iter_mut(&mut self) -> impl ParallelIterator<Item = (WorldId, WorldMut)> + '_ {
        self.0
            .sm
            .par_iter_mut()
            .map(|(k, v)| (WorldId(k), WorldMut::new(v)))
    }
}

/// A world on the server is a space for chunks, entities, and clients to
/// inhabit.
///
/// Worlds maintain a collection of chunks and entities that are a part of it.
/// For a chunk or entity to appear, it must be inserted into the world. Chunks
/// and entities can be in multiple worlds at the same time.
///
/// Deleted chunks and entities are automatically removed from worlds at the end
/// of each tick.
pub(crate) struct World {
    clients: Clients,
    entities: Entities,
    spatial_index: SpatialIndex,
    chunks: Chunks,
    meta: WorldMeta,
}

/// A bag of immutable references to the components of a world.
pub struct WorldRef<'a> {
    pub clients: &'a Clients,
    pub entities: &'a Entities,
    pub spatial_index: &'a SpatialIndex,
    pub chunks: &'a Chunks,
    pub meta: &'a WorldMeta,
}

impl<'a> WorldRef<'a> {
    pub(crate) fn new(w: &'a World) -> Self {
        Self {
            clients: &w.clients,
            entities: &w.entities,
            spatial_index: &w.spatial_index,
            chunks: &w.chunks,
            meta: &w.meta,
        }
    }
}

/// A bag of mutable references to the components of a world.
pub struct WorldMut<'a> {
    pub clients: ClientsMut<'a>,
    pub entities: EntitiesMut<'a>,
    pub spatial_index: SpatialIndexMut<'a>,
    pub chunks: ChunksMut<'a>,
    pub meta: WorldMetaMut<'a>,
}

impl<'a> WorldMut<'a> {
    pub(crate) fn new(w: &'a mut World) -> Self {
        WorldMut {
            clients: ClientsMut::new(&mut w.clients),
            entities: EntitiesMut::new(&mut w.entities),
            spatial_index: SpatialIndexMut::new(&mut w.spatial_index),
            chunks: ChunksMut::new(&mut w.chunks),
            meta: WorldMetaMut(&mut w.meta),
        }
    }

    pub fn immutable(&'a self) -> WorldRef<'a> {
        WorldRef {
            clients: &self.clients,
            entities: &self.entities,
            spatial_index: &self.spatial_index,
            chunks: &self.chunks,
            meta: &self.meta,
        }
    }
}

pub struct WorldMeta {
    dimension: DimensionId,
    is_flat: bool,
}

impl WorldMeta {
    pub fn dimension(&self) -> DimensionId {
        self.dimension
    }

    pub fn is_flat(&self) -> bool {
        self.is_flat
    }
}

pub struct WorldMetaMut<'a>(&'a mut WorldMeta);

impl<'a> Deref for WorldMetaMut<'a> {
    type Target = WorldMeta;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> WorldMetaMut<'a> {
    pub fn set_flat(&mut self, flat: bool) {
        self.0.is_flat = flat;
    }
}
