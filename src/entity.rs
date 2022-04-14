use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::iter::FusedIterator;

use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use rayon::iter::{IntoParallelRefIterator, IntoParallelRefMutIterator, ParallelIterator};
use uuid::Uuid;

use crate::chunk::ChunkPos;
use crate::client::MaybeClient;
use crate::component::{
    ComponentStore, Components, ComponentsMut, Error, Id, IdData, IdRaw, ZippedComponents,
    ZippedComponentsRaw,
};
use crate::{Aabb, WorldId};

pub mod appearance;

pub use appearance::Appearance;

pub struct EntityStore {
    comps: ComponentStore<EntityId>,
    uuids: Vec<Uuid>,
    clients: RwLock<Vec<MaybeClient>>,
    appearances: RwLock<Vec<Appearance>>,
    old_appearances: Vec<Appearance>,
    uuid_to_entity: HashMap<Uuid, EntityId>,
    /// Maps chunk positions to the set of all entities with bounding volumes
    /// intersecting that chunk.
    partition: HashMap<(WorldId, ChunkPos), Vec<EntityId>>,
}

impl EntityStore {
    pub(crate) fn new() -> Self {
        Self {
            comps: ComponentStore::new(),
            uuids: Vec::new(),
            clients: RwLock::new(Vec::new()),
            appearances: RwLock::new(Vec::new()),
            old_appearances: Vec::new(),
            uuid_to_entity: HashMap::new(),
            partition: HashMap::new(),
        }
    }

    /// Gets the [`EntityId`] of the entity with the given UUID in an efficient
    /// manner.
    ///
    /// Returns `None` if there is no entity with the provided UUID. Returns
    /// `Some` otherwise.
    pub fn with_uuid(&self, uuid: Uuid) -> Option<EntityId> {
        self.uuid_to_entity.get(&uuid).cloned()
    }

    /// Spawns a new entity with the provided appearance. The new entity's
    /// [`EntityId`] is returned.
    pub fn create(&mut self, appearance: impl Into<Appearance>) -> EntityId {
        let app = appearance.into();
        loop {
            let uuid = Uuid::from_bytes(rand::random());
            if let Some(e) = self.create_with_uuid(app.clone(), uuid) {
                return e;
            }
        }
    }

    /// Like [`create`](Entities::create), but requires specifying the new
    /// entity's UUID. This is useful for deserialization.
    ///
    /// The provided UUID must not conflict with an existing entity UUID. If it
    /// does, `None` is returned and the entity is not spawned.
    pub fn create_with_uuid(
        &mut self,
        appearance: impl Into<Appearance>,
        uuid: Uuid,
    ) -> Option<EntityId> {
        match self.uuid_to_entity.entry(uuid) {
            Entry::Occupied(_) => None,
            Entry::Vacant(ve) => {
                let app = appearance.into();

                let entity = self.comps.create_item();

                ve.insert(entity);

                if let (Some(aabb), Some(world)) = (app.aabb(), app.world()) {
                    self.partition_insert(entity, world, aabb);
                }

                let idx = entity.0.idx as usize;
                if idx >= self.uuids.len() {
                    self.uuids.push(uuid);
                    self.clients.get_mut().push(MaybeClient(None));
                    self.appearances.get_mut().push(app.clone());
                    self.old_appearances.push(app);
                } else {
                    self.uuids[idx] = uuid;
                    self.clients.get_mut()[idx].0 = None;
                    self.appearances.get_mut()[idx] = app.clone();
                    self.old_appearances[idx] = app;
                }

                Some(entity)
            }
        }
    }

    pub fn delete(&mut self, entity: EntityId) -> bool {
        if self.comps.delete_item(entity) {
            let idx = entity.0.idx as usize;

            self.uuid_to_entity
                .remove(&self.uuids[idx])
                .expect("UUID should have been in UUID map");

            self.clients.get_mut()[idx].0 = None;

            let app = &self.appearances.get_mut()[idx];
            if let (Some(aabb), Some(world)) = (app.aabb(), app.world()) {
                self.partition_remove(entity, world, aabb);
            }
            true
        } else {
            false
        }
    }

    /// Returns the number of live entities.
    pub fn count(&self) -> usize {
        self.comps.count()
    }

    pub fn is_valid(&self, entity: EntityId) -> bool {
        self.comps.is_valid(entity)
    }

    pub fn get<Z>(&self, z: Z, entity: EntityId) -> Option<Z::Item>
    where
        Z: ZippedComponents<Id = EntityId>,
    {
        self.comps.get(z, entity)
    }

    pub fn iter<'a, Z>(&'a self, z: Z) -> impl FusedIterator<Item = (EntityId, Z::Item)> + 'a
    where
        Z: ZippedComponents<Id = EntityId> + 'a,
    {
        self.comps.iter(z)
    }

    pub fn par_iter<'a, Z>(&'a self, z: Z) -> impl ParallelIterator<Item = (EntityId, Z::Item)> + 'a
    where
        Z: ZippedComponents<Id = EntityId> + 'a,
    {
        self.comps.par_iter(z)
    }

    pub fn ids(&self) -> impl FusedIterator<Item = EntityId> + Clone + '_ {
        self.comps.ids()
    }

    pub fn par_ids(&self) -> impl ParallelIterator<Item = EntityId> + Clone + '_ {
        self.comps.par_ids()
    }

    pub fn uuids(&self) -> Uuids {
        Uuids { uuids: &self.uuids }
    }

    pub fn clients(&self) -> Result<Clients, Error> {
        Ok(Clients {
            clients: self.clients.try_read().ok_or(Error::NoReadAccess)?,
        })
    }

    pub fn clients_mut(&self) -> Result<ClientsMut, Error> {
        Ok(ClientsMut {
            clients: self.clients.try_write().ok_or(Error::NoWriteAccess)?,
        })
    }

    pub fn appearances(&self) -> Result<Appearances, Error> {
        Ok(Appearances {
            appearances: self.appearances.try_read().ok_or(Error::NoReadAccess)?,
        })
    }

    pub fn appearances_mut(&self) -> Result<AppearancesMut, Error> {
        Ok(AppearancesMut {
            appearances: self.appearances.try_write().ok_or(Error::NoWriteAccess)?,
        })
    }

    pub fn old_appearances(&self) -> OldAppearances {
        OldAppearances {
            old_appearances: &self.old_appearances,
        }
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
    ) -> Result<Components<C, EntityId>, Error> {
        self.comps.components::<C>()
    }

    pub fn components_mut<C: 'static + Send + Sync + Default>(
        &self,
    ) -> Result<ComponentsMut<C, EntityId>, Error> {
        self.comps.components_mut::<C>()
    }

    fn partition_insert(&mut self, entity: EntityId, world: WorldId, aabb: Aabb<f64, 3>) {
        let min_corner = ChunkPos::from_xz(aabb.min().xz());
        let max_corner = ChunkPos::from_xz(aabb.max().xz());

        for z in min_corner.z..=max_corner.z {
            for x in min_corner.x..=max_corner.x {
                self.partition_insert_at(entity, world, ChunkPos { x, z })
            }
        }
    }

    fn partition_insert_at(&mut self, entity: EntityId, world: WorldId, pos: ChunkPos) {
        match self.partition.entry((world, pos)) {
            Entry::Occupied(mut oe) => {
                debug_assert!(
                    !oe.get_mut().contains(&entity),
                    "spatial partition: entity already present"
                );
                oe.get_mut().push(entity);
            }
            Entry::Vacant(ve) => {
                ve.insert(vec![entity]);
            }
        }
    }

    fn partition_remove(&mut self, entity: EntityId, world: WorldId, aabb: Aabb<f64, 3>) {
        let min_corner = ChunkPos::from_xz(aabb.min().xz());
        let max_corner = ChunkPos::from_xz(aabb.max().xz());

        for z in min_corner.z..=max_corner.z {
            for x in min_corner.x..=max_corner.x {
                self.partition_remove_at(entity, world, ChunkPos::new(x, z));
            }
        }
    }

    fn partition_remove_at(&mut self, entity: EntityId, world: WorldId, pos: ChunkPos) {
        let errmsg = "spatial partition: entity removal failed";
        match self.partition.entry((world, pos)) {
            Entry::Occupied(mut oe) => {
                let v = oe.get_mut();
                let idx = v.iter().position(|e| *e == entity).expect(errmsg);
                v.swap_remove(idx);

                if v.is_empty() {
                    oe.remove();
                }
            }
            Entry::Vacant(_) => panic!("{errmsg}"),
        }
    }

    fn partition_modify(
        &mut self,
        entity: EntityId,
        old_world: WorldId,
        old_aabb: Aabb<f64, 3>,
        new_world: WorldId,
        new_aabb: Aabb<f64, 3>,
    ) {
        if old_world != new_world {
            self.partition_remove(entity, old_world, old_aabb);
            self.partition_insert(entity, new_world, new_aabb);
        } else {
            let old_min_corner = ChunkPos::from_xz(old_aabb.min().xz());
            let old_max_corner = ChunkPos::from_xz(old_aabb.max().xz());
            let new_min_corner = ChunkPos::from_xz(new_aabb.min().xz());
            let new_max_corner = ChunkPos::from_xz(new_aabb.max().xz());

            for z in new_min_corner.z..=new_max_corner.z {
                for x in new_min_corner.x..=new_max_corner.x {
                    if x < old_min_corner.x
                        || x > old_max_corner.x
                        || z < old_min_corner.z
                        || z > old_max_corner.z
                    {
                        self.partition_insert_at(entity, old_world, ChunkPos::new(x, z));
                    }
                }
            }

            for z in old_min_corner.z..=old_max_corner.z {
                for x in old_min_corner.x..=old_max_corner.x {
                    if x < new_min_corner.x
                        || x > new_max_corner.x
                        || z < new_min_corner.z
                        || z > new_max_corner.z
                    {
                        self.partition_remove_at(entity, old_world, ChunkPos::new(x, z))
                    }
                }
            }
        }
    }

    /// Returns an iterator over all entities with bounding volumes intersecting
    /// the given AABB in an arbitrary order.
    pub fn intersecting_aabb(
        &self,
        world: WorldId,
        aabb: Aabb<f64, 3>,
    ) -> impl FusedIterator<Item = EntityId> + '_ {
        let min_corner = ChunkPos::from_xz(aabb.min().xz());
        let max_corner = ChunkPos::from_xz(aabb.max().xz());
        (min_corner.z..=max_corner.z).flat_map(move |z| {
            (min_corner.x..=max_corner.x).flat_map(move |x| {
                self.partition
                    .get(&(world, ChunkPos::new(x, z)))
                    .into_iter()
                    .flat_map(move |v| {
                        v.iter().cloned().filter(move |&e| {
                            self.get(&self.old_appearances(), e)
                                .expect("spatial partition contains expired entity")
                                .aabb()
                                .expect("spatial partition contains entity without AABB")
                                .collides_with_aabb(&aabb)
                        })
                    })
            })
        })
    }

    pub(crate) fn update_old_appearances(&mut self) {
        for (old, new) in self
            .old_appearances
            .iter_mut()
            .zip(self.appearances.get_mut().iter())
        {
            old.clone_from(new);
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, Debug)]
pub struct EntityId(IdData);

impl IdRaw for EntityId {
    fn from_data(data: IdData) -> Self {
        Self(data)
    }

    fn to_data(self) -> IdData {
        self.0
    }
}

impl EntityId {
    /// The vaule of the default `EntityId` which always refers to an expired
    /// entity.
    pub const NULL: Self = Self(IdData::NULL);

    pub(crate) fn to_network_id(self) -> i32 {
        self.0.idx as i32
    }
}

impl Id for EntityId {}

/// A built-in component collection containing the UUID of the entities.
///
/// The default value for this component is a random unassigned UUID. UUIDs
/// cannot be modified after an entity is created.
///
/// TODO: describe the UUID for players.
pub struct Uuids<'a> {
    uuids: &'a Vec<Uuid>,
}

impl<'a, 'b> ZippedComponentsRaw for &'b Uuids<'a> {
    type RawItem = Uuid;
    type RawIter = std::iter::Cloned<std::slice::Iter<'b, Uuid>>;
    type RawParIter = rayon::iter::Cloned<rayon::slice::Iter<'b, Uuid>>;

    fn raw_get(self, idx: usize) -> Self::RawItem {
        self.uuids[idx]
    }

    fn raw_iter(self) -> Self::RawIter {
        self.uuids.iter().cloned()
    }

    fn raw_par_iter(self) -> Self::RawParIter {
        self.uuids.par_iter().cloned()
    }
}

impl<'a, 'b> ZippedComponents for &'b Uuids<'a> {
    type Id = EntityId;
    type Item = Uuid;
}

/// A built-in component collection containing the clients that entites are
/// backed by, if any.
///
/// When a client joins the server, a new entity is created which is backed by
/// the new client. However, when a client is disconnected, the entity which
/// they inhabited is _not_ automatically deleted.
///
/// Deleting the associated entity while the client is still connected will
/// immediately disconnect the client.
///
/// The default value of this component will not contain a client and all calls
/// to [`get`](Self::get) and [`get_mut`](Self::get_mut) will return `None`.

pub struct Clients<'a> {
    // TODO: box the clients
    clients: RwLockReadGuard<'a, Vec<MaybeClient>>,
}

impl<'a, 'b> ZippedComponentsRaw for &'b Clients<'a> {
    type RawItem = &'b MaybeClient;
    type RawIter = std::slice::Iter<'b, MaybeClient>;
    type RawParIter = rayon::slice::Iter<'b, MaybeClient>;

    fn raw_get(self, idx: usize) -> Self::RawItem {
        &self.clients[idx]
    }

    fn raw_iter(self) -> Self::RawIter {
        self.clients.iter()
    }

    fn raw_par_iter(self) -> Self::RawParIter {
        self.clients.par_iter()
    }
}

impl<'a, 'b> ZippedComponents for &'b Clients<'a> {
    type Id = EntityId;
    type Item = &'b MaybeClient;
}

pub struct ClientsMut<'a> {
    clients: RwLockWriteGuard<'a, Vec<MaybeClient>>,
}

impl<'a, 'b> ZippedComponentsRaw for &'b ClientsMut<'a> {
    type RawItem = &'b MaybeClient;
    type RawIter = std::slice::Iter<'b, MaybeClient>;
    type RawParIter = rayon::slice::Iter<'b, MaybeClient>;

    fn raw_get(self, idx: usize) -> Self::RawItem {
        &self.clients[idx]
    }

    fn raw_iter(self) -> Self::RawIter {
        self.clients.iter()
    }

    fn raw_par_iter(self) -> Self::RawParIter {
        self.clients.par_iter()
    }
}

impl<'a, 'b> ZippedComponents for &'b ClientsMut<'a> {
    type Id = EntityId;
    type Item = &'b MaybeClient;
}

impl<'a, 'b> ZippedComponentsRaw for &'b mut ClientsMut<'a> {
    type RawItem = &'b mut MaybeClient;
    type RawIter = std::slice::IterMut<'b, MaybeClient>;
    type RawParIter = rayon::slice::IterMut<'b, MaybeClient>;

    fn raw_get(self, idx: usize) -> Self::RawItem {
        &mut self.clients[idx]
    }

    fn raw_iter(self) -> Self::RawIter {
        self.clients.iter_mut()
    }

    fn raw_par_iter(self) -> Self::RawParIter {
        self.clients.par_iter_mut()
    }
}

impl<'a, 'b> ZippedComponents for &'b mut ClientsMut<'a> {
    type Id = EntityId;
    type Item = &'b mut MaybeClient;
}

pub struct Appearances<'a> {
    appearances: RwLockReadGuard<'a, Vec<Appearance>>,
}

impl<'a, 'b> ZippedComponentsRaw for &'b Appearances<'a> {
    type RawItem = &'b Appearance;
    type RawIter = std::slice::Iter<'b, Appearance>;
    type RawParIter = rayon::slice::Iter<'b, Appearance>;

    fn raw_get(self, idx: usize) -> Self::RawItem {
        &self.appearances[idx]
    }

    fn raw_iter(self) -> Self::RawIter {
        self.appearances.iter()
    }

    fn raw_par_iter(self) -> Self::RawParIter {
        self.appearances.par_iter()
    }
}

impl<'a, 'b> ZippedComponents for &'b Appearances<'a> {
    type Id = EntityId;
    type Item = &'b Appearance;
}

pub struct AppearancesMut<'a> {
    appearances: RwLockWriteGuard<'a, Vec<Appearance>>,
}

impl<'a, 'b> ZippedComponentsRaw for &'b AppearancesMut<'a> {
    type RawItem = &'b Appearance;
    type RawIter = std::slice::Iter<'b, Appearance>;
    type RawParIter = rayon::slice::Iter<'b, Appearance>;

    fn raw_get(self, idx: usize) -> Self::RawItem {
        &self.appearances[idx]
    }

    fn raw_iter(self) -> Self::RawIter {
        self.appearances.iter()
    }

    fn raw_par_iter(self) -> Self::RawParIter {
        self.appearances.par_iter()
    }
}

impl<'a, 'b> ZippedComponents for &'b AppearancesMut<'a> {
    type Id = EntityId;
    type Item = &'b Appearance;
}

impl<'a, 'b> ZippedComponentsRaw for &'b mut AppearancesMut<'a> {
    type RawItem = &'b mut Appearance;
    type RawIter = std::slice::IterMut<'b, Appearance>;
    type RawParIter = rayon::slice::IterMut<'b, Appearance>;

    fn raw_get(self, idx: usize) -> Self::RawItem {
        &mut self.appearances[idx]
    }

    fn raw_iter(self) -> Self::RawIter {
        self.appearances.iter_mut()
    }

    fn raw_par_iter(self) -> Self::RawParIter {
        self.appearances.par_iter_mut()
    }
}

impl<'a, 'b> ZippedComponents for &'b mut AppearancesMut<'a> {
    type Id = EntityId;
    type Item = &'b mut Appearance;
}

/// Contains a snapshot of an entity's [`Appearance`] as it existed at the end
/// of the previous tick.
pub struct OldAppearances<'a> {
    old_appearances: &'a Vec<Appearance>,
}

impl<'a, 'b> ZippedComponentsRaw for &'b OldAppearances<'a> {
    type RawItem = &'b Appearance;
    type RawIter = std::slice::Iter<'b, Appearance>;
    type RawParIter = rayon::slice::Iter<'b, Appearance>;

    fn raw_get(self, idx: usize) -> Self::RawItem {
        &self.old_appearances[idx]
    }

    fn raw_iter(self) -> Self::RawIter {
        self.old_appearances.iter()
    }

    fn raw_par_iter(self) -> Self::RawParIter {
        self.old_appearances.par_iter()
    }
}

impl<'a, 'b> ZippedComponents for &'b OldAppearances<'a> {
    type Id = EntityId;
    type Item = &'b Appearance;
}

#[cfg(test)]
mod tests {
    use appearance::Player;

    use super::*;
    use crate::glm;

    // TODO: better test: spawn a bunch of random entities, spawn a random AABB,
    // assert collides_with_aabb consistency.

    #[test]
    fn space_partition() {
        let mut entities = EntityStore::new();

        let ids = [(16.0, 16.0, 16.0), (8.0, 8.0, 8.0), (10.0, 50.0, 10.0)]
            .into_iter()
            .map(|(x, y, z)| entities.create(Player::new(glm::vec3(x, y, z), WorldId::NULL)))
            .collect::<Vec<_>>();

        let outside = *ids.last().unwrap();

        assert!(entities
            .intersecting_aabb(
                WorldId::NULL,
                Aabb::new(glm::vec3(8.0, 8.0, 8.0), glm::vec3(16.0, 16.0, 16.0)),
            )
            .all(|id| ids.contains(&id) && id != outside));
    }
}
