use std::num::Wrapping;

use bevy_ecs::prelude::*;
use rustc_hash::FxHashMap;
use tracing::warn;

use super::EntityId;

/// A [`Resource`] which maintains information about all spawned Minecraft
/// entities.
#[derive(Resource, Debug)]
pub struct EntityManager {
    /// Maps protocol IDs to ECS entities.
    pub(super) id_to_entity: FxHashMap<i32, Entity>,
    next_id: Wrapping<i32>,
}

impl EntityManager {
    pub(super) fn new() -> Self {
        Self {
            id_to_entity: FxHashMap::default(),
            next_id: Wrapping(1), // Skip 0.
        }
    }

    /// Returns the next unique entity ID and increments the counter.
    pub fn next_id(&mut self) -> EntityId {
        if self.next_id.0 == 0 {
            warn!("entity ID overflow!");
            // ID 0 is reserved for clients, so skip over it.
            self.next_id.0 = 1;
        }

        let id = EntityId(self.next_id.0);

        self.next_id += 1;

        id
    }

    /// Gets the entity with the given entity ID.
    pub fn get_by_id(&self, entity_id: i32) -> Option<Entity> {
        self.id_to_entity.get(&entity_id).cloned()
    }
}
