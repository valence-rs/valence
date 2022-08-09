//! A space on a server for objects to occupy.

use std::iter::FusedIterator;

use rayon::iter::ParallelIterator;

use crate::chunk::Chunks;
use crate::config::Config;
use crate::dimension::DimensionId;
use crate::server::SharedServer;
use crate::slab_versioned::{Key, VersionedSlab};
use crate::spatial_index::SpatialIndex;

/// A container for all [`World`]s on a [`Server`](crate::server::Server).
pub struct Worlds<C: Config> {
    slab: VersionedSlab<World<C>>,
    server: SharedServer<C>,
}

/// An identifier for a [`World`] on the server.
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

impl<C: Config> Worlds<C> {
    pub(crate) fn new(server: SharedServer<C>) -> Self {
        Self {
            slab: VersionedSlab::new(),
            server,
        }
    }

    /// Creates a new world on the server with the provided dimension. A
    /// reference to the world along with its ID is returned.
    pub fn insert(&mut self, dim: DimensionId, state: C::WorldState) -> (WorldId, &mut World<C>) {
        let (id, world) = self.slab.insert(World {
            state,
            spatial_index: SpatialIndex::new(),
            chunks: Chunks::new(self.server.clone(), dim),
            meta: WorldMeta { dimension: dim },
        });

        (WorldId(id), world)
    }

    /// Deletes a world from the server.
    ///
    /// Note that entities located in the world are not deleted themselves.
    /// Additionally, any clients that are still in the deleted world at the end
    /// of the tick are disconnected.
    pub fn remove(&mut self, world: WorldId) -> bool {
        self.slab.remove(world.0).is_some()
    }

    /// Deletes all worlds from the server (as if by [`Self::delete`]) for which
    /// `f` returns `true`.
    ///
    /// All worlds are visited in an unspecified order.
    pub fn retain(&mut self, mut f: impl FnMut(WorldId, &mut World<C>) -> bool) {
        self.slab.retain(|k, v| f(WorldId(k), v))
    }

    /// Returns the number of worlds on the server.
    pub fn len(&self) -> usize {
        self.slab.len()
    }

    /// Returns a shared reference to the world with the given ID. If
    /// the ID is invalid, then `None` is returned.
    pub fn get(&self, world: WorldId) -> Option<&World<C>> {
        self.slab.get(world.0)
    }

    /// Returns an exclusive reference to the world with the given ID. If the
    /// ID is invalid, then `None` is returned.
    pub fn get_mut(&mut self, world: WorldId) -> Option<&mut World<C>> {
        self.slab.get_mut(world.0)
    }

    /// Returns an immutable iterator over all worlds on the server in an
    /// unspecified order.
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (WorldId, &World<C>)> + FusedIterator + Clone + '_ {
        self.slab.iter().map(|(k, v)| (WorldId(k), v))
    }

    /// Returns a mutable iterator over all worlds on the server in an
    /// unspecified order.
    pub fn iter_mut(
        &mut self,
    ) -> impl ExactSizeIterator<Item = (WorldId, &mut World<C>)> + FusedIterator + '_ {
        self.slab.iter_mut().map(|(k, v)| (WorldId(k), v))
    }

    /// Returns a parallel immutable iterator over all worlds on the server in
    /// an unspecified order.
    pub fn par_iter(&self) -> impl ParallelIterator<Item = (WorldId, &World<C>)> + Clone + '_ {
        self.slab.par_iter().map(|(k, v)| (WorldId(k), v))
    }

    /// Returns a parallel mutable iterator over all worlds on the server in an
    /// unspecified order.
    pub fn par_iter_mut(&mut self) -> impl ParallelIterator<Item = (WorldId, &mut World<C>)> + '_ {
        self.slab.par_iter_mut().map(|(k, v)| (WorldId(k), v))
    }
}

/// A space for chunks, entities, and clients to occupy.
pub struct World<C: Config> {
    /// Custom state.
    pub state: C::WorldState,
    /// Contains all of the entities in this world.
    pub spatial_index: SpatialIndex,
    /// All of the chunks in this world.
    pub chunks: Chunks<C>,
    /// This world's metadata.
    pub meta: WorldMeta,
}

/// Contains miscellaneous world state.
pub struct WorldMeta {
    dimension: DimensionId,
}

impl WorldMeta {
    /// Gets the dimension the world was created with.
    pub fn dimension(&self) -> DimensionId {
        self.dimension
    }
}
