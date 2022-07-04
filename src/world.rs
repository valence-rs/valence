use std::iter::FusedIterator;

use rayon::iter::ParallelIterator;

use crate::player_list::PlayerList;
use crate::slotmap::{Key, SlotMap};
use crate::{Chunks, DimensionId, SharedServer, SpatialIndex};

pub struct Worlds {
    sm: SlotMap<World>,
    server: SharedServer,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct WorldId(Key);

impl WorldId {
    pub const NULL: Self = Self(Key::NULL);
}

impl Worlds {
    pub(crate) fn new(server: SharedServer) -> Self {
        Self {
            sm: SlotMap::new(),
            server,
        }
    }

    pub fn create(&mut self, dim: DimensionId) -> (WorldId, &mut World) {
        let (id, world) = self.sm.insert(World {
            spatial_index: SpatialIndex::new(),
            chunks: Chunks::new(self.server.clone(), dim),
            meta: WorldMeta {
                dimension: dim,
                is_flat: false,
                player_list: PlayerList::new(),
            },
        });

        (WorldId(id), world)
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

    pub fn count(&self) -> usize {
        self.sm.len()
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
    pub spatial_index: SpatialIndex,
    pub chunks: Chunks,
    pub meta: WorldMeta,
}

pub struct WorldMeta {
    dimension: DimensionId,
    is_flat: bool,
    player_list: PlayerList,
    // TODO: time, weather
}

impl WorldMeta {
    pub fn dimension(&self) -> DimensionId {
        self.dimension
    }

    pub fn is_flat(&self) -> bool {
        self.is_flat
    }

    pub fn set_flat(&mut self, flat: bool) {
        self.is_flat = flat;
    }

    pub fn player_list(&self) -> &PlayerList {
        &self.player_list
    }

    pub fn player_list_mut(&mut self) -> &mut PlayerList {
        &mut self.player_list
    }

    pub(crate) fn update(&mut self) {
        self.player_list.update();
    }
}
