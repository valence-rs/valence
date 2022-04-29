pub mod meta;
pub mod types;

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::iter::FusedIterator;

use rayon::iter::ParallelIterator;
use uuid::Uuid;

use crate::chunk::ChunkPos;
use crate::glm::DVec3;
use crate::slotmap::{Key, SlotMap};
use crate::{Aabb, Id, WorldId};

pub struct EntityStore {
    sm: SlotMap<Entity>,
    uuid_to_entity: HashMap<Uuid, EntityId>,
    /// Maps chunk positions to the set of all entities with bounding volumes
    /// intersecting that chunk.
    partition: HashMap<(WorldId, ChunkPos), Vec<EntityId>>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct EntityId(Key);

impl Id for EntityId {
    fn idx(self) -> usize {
        self.0.index() as usize
    }
}

impl EntityId {
    pub(crate) fn to_network_id(self) -> i32 {
        // TODO: is ID 0 reserved?
        self.0.index() as i32
    }
}

pub struct Entity {
    data: EntityData,
    old_type: EntityType,
    new_position: DVec3,
    old_position: DVec3,
    new_world: Option<WorldId>,
    old_world: Option<WorldId>,
    uuid: Uuid,
}

impl Entity {
    pub fn data(&self) -> &EntityData {
        &self.data
    }

    pub fn typ(&self) -> EntityType {
        self.data.typ()
    }

    /// Changes the type of this entity.
    pub fn change_type(&mut self, new_type: EntityType) {
        todo!(); // TODO
    }

    fn hitbox(&self) -> Aabb<f64, 3> {
        // TODO
        Aabb::default()
    }
}

pub use types::{EntityData, EntityType};

impl EntityStore {
    pub(crate) fn new() -> Self {
        Self {
            sm: SlotMap::new(),
            uuid_to_entity: HashMap::new(),
            partition: HashMap::new(),
        }
    }

    /// Returns the number of live entities.
    pub fn count(&self) -> usize {
        self.sm.count()
    }

    /// Spawns a new entity with the default data. The new entity'd [`EntityId`]
    /// is returned.
    ///
    /// To actually see the new entity, set its position to somewhere nearby and
    /// [change its type](EntityData::change_type) to something visible.
    pub fn create(&mut self) -> EntityId {
        loop {
            let uuid = Uuid::from_bytes(rand::random());
            if let Some(entity) = self.create_with_uuid(uuid) {
                return entity;
            }
        }
    }

    /// Like [`create`](Entities::create), but requires specifying the new
    /// entity's UUID. This is useful for deserialization.
    ///
    /// The provided UUID must not conflict with an existing entity UUID. If it
    /// does, `None` is returned and the entity is not spawned.
    pub fn create_with_uuid(&mut self, uuid: Uuid) -> Option<EntityId> {
        match self.uuid_to_entity.entry(uuid) {
            Entry::Occupied(_) => None,
            Entry::Vacant(ve) => {
                let entity = EntityId(self.sm.insert(Entity {
                    data: EntityData::Marker(types::Marker::new()),
                    old_type: EntityType::Marker,
                    new_position: DVec3::default(),
                    old_position: DVec3::default(),
                    new_world: None,
                    old_world: None,
                    uuid,
                }));

                ve.insert(entity);

                // TODO: insert into partition.

                Some(entity)
            }
        }
    }

    /// Gets the [`EntityId`] of the entity with the given UUID in an efficient
    /// manner.
    ///
    /// Returns `None` if there is no entity with the provided UUID. Returns
    /// `Some` otherwise.
    pub fn get_with_uuid(&self, uuid: Uuid) -> Option<EntityId> {
        self.uuid_to_entity.get(&uuid).cloned()
    }

    pub fn delete(&mut self, entity: EntityId) -> bool {
        if let Some(e) = self.sm.remove(entity.0) {
            self.uuid_to_entity
                .remove(&e.uuid)
                .expect("UUID should have been in UUID map");

            // TODO: remove entity from partition.
            true
        } else {
            false
        }
    }

    pub fn retain(&mut self, mut f: impl FnMut(EntityId, &mut Entity) -> bool) {
        self.sm.retain(|k, v| f(EntityId(k), v))
    }

    pub fn get(&self, entity: EntityId) -> Option<&Entity> {
        self.sm.get(entity.0)
    }

    pub fn get_mut(&mut self, entity: EntityId) -> Option<&mut Entity> {
        self.sm.get_mut(entity.0)
    }

    pub fn iter(&self) -> impl FusedIterator<Item = (EntityId, &Entity)> + Clone + '_ {
        self.sm.iter().map(|(k, v)| (EntityId(k), v))
    }

    pub fn iter_mut(&mut self) -> impl FusedIterator<Item = (EntityId, &mut Entity)> + '_ {
        self.sm.iter_mut().map(|(k, v)| (EntityId(k), v))
    }

    pub fn par_iter(&self) -> impl ParallelIterator<Item = (EntityId, &Entity)> + Clone + '_ {
        self.sm.par_iter().map(|(k, v)| (EntityId(k), v))
    }

    pub fn par_iter_mut(&mut self) -> impl ParallelIterator<Item = (EntityId, &mut Entity)> + '_ {
        self.sm.par_iter_mut().map(|(k, v)| (EntityId(k), v))
    }

    pub(crate) fn from_network_id(&self, network_id: i32) -> Option<EntityId> {
        self.sm.key_at_index(network_id as usize).map(EntityId)
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
                            self.get(e)
                                .expect("spatial partition contains deleted entity")
                                .hitbox()
                                .collides_with_aabb(&aabb)
                        })
                    })
            })
        })
    }

    pub(crate) fn update(&mut self) {
        for (_, e) in self.iter_mut() {
            e.old_position = e.new_position;
            e.old_world = e.new_world;

            // TODO: update entity old_type.
            // TODO: clear changed bits in metadata.
        }
    }
}

//#[cfg(test)]
//mod tests {
//    use appearance::Player;
//
//    use super::*;
//    use crate::glm;
//
//    // TODO: better test: spawn a bunch of random entities, spawn a random
// AABB,    // assert collides_with_aabb consistency.
//
//    #[test]
//    fn space_partition() {
//        let mut entities = EntityStore::new();
//
//        let ids = [(16.0, 16.0, 16.0), (8.0, 8.0, 8.0), (10.0, 50.0, 10.0)]
//            .into_iter()
//            .map(|(x, y, z)| entities.create(Player::new(glm::vec3(x, y, z),
// WorldId::NULL)))            .collect::<Vec<_>>();
//
//        let outside = *ids.last().unwrap();
//
//        assert!(entities
//            .intersecting_aabb(
//                WorldId::NULL,
//                Aabb::new(glm::vec3(8.0, 8.0, 8.0), glm::vec3(16.0, 16.0,
// 16.0)),            )
//            .all(|id| ids.contains(&id) && id != outside));
//    }
//}
