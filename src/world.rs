use std::iter::FusedIterator;

use rayon::iter::ParallelIterator;

use crate::chunk::Chunks;
use crate::dimension::DimensionId;
use crate::player_list::PlayerList;
use crate::server::SharedServer;
use crate::slotmap::{Key, SlotMap};
use crate::spatial_index::SpatialIndex;

pub struct Worlds {
    sm: SlotMap<World>,
    server: SharedServer,
}

/// A key for a [`World`] on the server.
///
/// World IDs are either _valid_ or _invalid_. Valid world IDs point to
/// worlds that have not been deleted, while invalid IDs point to those that
/// have. Once an ID becomes invalid, it will never become valid again.
///
/// The [`Ord`] instance on this type is correct but otherwise unspecified. This
/// is useful for storing IDs in containers such as
/// [`BTreeMap`](std::collections::BTreeMap).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct WorldId(Key);

impl WorldId {
    /// The value of the default world ID which is always invalid.
    pub const NULL: Self = Self(Key::NULL);
}

impl Worlds {
    pub(crate) fn new(server: SharedServer) -> Self {
        Self {
            sm: SlotMap::new(),
            server,
        }
    }

    /// Creates a new world on the server with the provided dimension. A
    /// reference to the world along with its ID is returned.
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

    /// Deletes a world from the server.
    ///
    /// Note that entities located in the world are not deleted themselves.
    /// Additionally, any clients that are still in the deleted world at the end
    /// of the tick are disconnected.
    pub fn delete(&mut self, world: WorldId) -> bool {
        self.sm.remove(world.0).is_some()
    }

    /// Deletes all worlds from the server (as if by [`Self::delete`]) for which
    /// `f` returns `true`.
    ///
    /// All worlds are visited in an unspecified order.
    pub fn retain(&mut self, mut f: impl FnMut(WorldId, &mut World) -> bool) {
        self.sm.retain(|k, v| f(WorldId(k), v))
    }

    /// Returns the number of worlds on the server.
    pub fn count(&self) -> usize {
        self.sm.len()
    }

    /// Returns a shared reference to the world with the given ID. If
    /// the ID is invalid, then `None` is returned.
    pub fn get(&self, world: WorldId) -> Option<&World> {
        self.sm.get(world.0)
    }

    /// Returns an exclusive reference to the world with the given ID. If the
    /// ID is invalid, then `None` is returned.
    pub fn get_mut(&mut self, world: WorldId) -> Option<&mut World> {
        self.sm.get_mut(world.0)
    }

    /// Returns an immutable iterator over all worlds on the server in an
    /// unspecified order.
    pub fn iter(&self) -> impl FusedIterator<Item = (WorldId, &World)> + Clone + '_ {
        self.sm.iter().map(|(k, v)| (WorldId(k), v))
    }

    /// Returns a mutable iterator over all worlds on the server in an
    /// unspecified ordder.
    pub fn iter_mut(&mut self) -> impl FusedIterator<Item = (WorldId, &mut World)> + '_ {
        self.sm.iter_mut().map(|(k, v)| (WorldId(k), v))
    }

    /// Returns a parallel immutable iterator over all worlds on the server in
    /// an unspecified order.
    pub fn par_iter(&self) -> impl ParallelIterator<Item = (WorldId, &World)> + Clone + '_ {
        self.sm.par_iter().map(|(k, v)| (WorldId(k), v))
    }

    /// Returns a parallel mutable iterator over all worlds on the server in an
    /// unspecified order.
    pub fn par_iter_mut(&mut self) -> impl ParallelIterator<Item = (WorldId, &mut World)> + '_ {
        self.sm.par_iter_mut().map(|(k, v)| (WorldId(k), v))
    }
}

/// A space for chunks, entities, and clients to occupy.
pub struct World {
    /// Contains all of the entities in this world.
    pub spatial_index: SpatialIndex,
    /// All of the chunks in this world.
    pub chunks: Chunks,
    /// This world's metadata.
    pub meta: WorldMeta,
}

/// Contains miscellaneous world state.
pub struct WorldMeta {
    dimension: DimensionId,
    is_flat: bool,
    player_list: PlayerList,
    // TODO: time, weather
}

impl WorldMeta {
    /// Gets the dimension the world was created with.
    pub fn dimension(&self) -> DimensionId {
        self.dimension
    }

    /// Gets if this world is considered a superflat world. Superflat worlds
    /// have a horizon line at y=0.
    pub fn is_flat(&self) -> bool {
        self.is_flat
    }

    /// Sets if this world is considered a superflat world. Superflat worlds
    /// have a horizon line at y=0.
    ///
    /// Clients already in the world must be respawned to see any changes.
    pub fn set_flat(&mut self, flat: bool) {
        self.is_flat = flat;
    }

    /// Returns a shared reference to the world's
    /// [`PlayerList`](crate::player_list::PlayerList).
    pub fn player_list(&self) -> &PlayerList {
        &self.player_list
    }

    /// Returns an exclusive reference to the world's
    /// [`PlayerList`](crate::player_list::PlayerList).
    pub fn player_list_mut(&mut self) -> &mut PlayerList {
        &mut self.player_list
    }

    pub(crate) fn update(&mut self) {
        self.player_list.update();
    }
}
