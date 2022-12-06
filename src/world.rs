//! A space on a server for objects to occupy.

use std::iter::FusedIterator;
use std::ops::{Deref, DerefMut};

use rayon::iter::ParallelIterator;

use crate::chunk::Chunks;
use crate::config::Config;
use crate::dimension::DimensionId;
use crate::server::SharedServer;
use crate::slab_versioned::{Key, VersionedSlab};

/// A container for all [`World`]s on a [`Server`](crate::server::Server).
pub struct Worlds<C: Config> {
    slab: VersionedSlab<World<C>>,
    shared: SharedServer<C>,
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
    pub(crate) fn new(shared: SharedServer<C>) -> Self {
        Self {
            slab: VersionedSlab::new(),
            shared,
        }
    }

    /// Creates a new world on the server with the provided dimension. A
    /// reference to the world along with its ID is returned.
    pub fn insert(
        &mut self,
        dimension: DimensionId,
        state: C::WorldState,
    ) -> (WorldId, &mut World<C>) {
        let dim = self.shared.dimension(dimension);

        let (id, world) = self.slab.insert(World {
            state,
            chunks: Chunks::new(dim.height, dim.min_y),
            dimension,
            deleted: false,
        });

        (WorldId(id), world)
    }

    /// Deletes a world from the server.
    ///
    /// Note that any entities located in the world are not deleted.
    /// Additionally, clients that are still in the deleted world at the end
    /// of the tick are disconnected.
    pub fn remove(&mut self, world: WorldId) -> Option<C::WorldState> {
        self.slab.remove(world.0).map(|w| w.state)
    }

    /// Removes all worlds from the server for which `f` returns `false`.
    ///
    /// All worlds are visited in an unspecified order.
    pub fn retain(&mut self, mut f: impl FnMut(WorldId, &mut World<C>) -> bool) {
        self.slab.retain(|k, v| f(WorldId(k), v))
    }

    /// Returns the number of worlds on the server.
    pub fn len(&self) -> usize {
        self.slab.len()
    }

    /// Returns `true` if there are no worlds on the server.
    pub fn is_empty(&self) -> bool {
        self.slab.len() == 0
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

    /// Returns an iterator over all worlds on the server in an unspecified
    /// order.
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

    /// Returns a parallel iterator over all worlds on the server in an
    /// unspecified order.
    pub fn par_iter(&self) -> impl ParallelIterator<Item = (WorldId, &World<C>)> + Clone + '_ {
        self.slab.par_iter().map(|(k, v)| (WorldId(k), v))
    }

    /// Returns a parallel mutable iterator over all worlds on the server in an
    /// unspecified order.
    pub fn par_iter_mut(&mut self) -> impl ParallelIterator<Item = (WorldId, &mut World<C>)> + '_ {
        self.slab.par_iter_mut().map(|(k, v)| (WorldId(k), v))
    }

    pub(crate) fn update(&mut self) {
        self.slab.retain(|_, world| !world.deleted);

        self.par_iter_mut().for_each(|(_, world)| {
            world.chunks.update();
        });
    }
}

/// A space for chunks, entities, and clients to occupy.
pub struct World<C: Config> {
    /// Custom state.
    pub state: C::WorldState,
    pub chunks: Chunks<C>,
    dimension: DimensionId,
    deleted: bool,
}

impl<C: Config> Deref for World<C> {
    type Target = C::WorldState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<C: Config> DerefMut for World<C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

impl<C: Config> World<C> {
    /// Gets the dimension the world was created with.
    pub fn dimension(&self) -> DimensionId {
        self.dimension
    }

    pub fn deleted(&self) -> bool {
        self.deleted
    }

    /// Whether or not this world should be marked as deleted. Deleted worlds
    /// are removed from the server at the end of the tick.
    ///
    /// Note that any entities located in the world are not deleted and their
    /// location will not change. Additionally, clients that are still in
    /// the deleted world at the end of the tick are disconnected.
    pub fn set_deleted(&mut self, deleted: bool) {
        self.deleted = deleted;
    }
}
