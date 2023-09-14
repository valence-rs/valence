use std::collections::hash_map::Entry;

use bevy_ecs::prelude::*;
use rustc_hash::FxHashMap;
use valence_protocol::ChunkPos;

/// Maps chunk positions to the set of clients in view of the chunk.
#[derive(Component, Default, Debug)]
pub struct ChunkViewIndex {
    map: FxHashMap<ChunkPos, Vec<Entity>>,
}

impl ChunkViewIndex {
    pub fn get(
        &self,
        pos: impl Into<ChunkPos>,
    ) -> impl ExactSizeIterator<Item = Entity> + Clone + '_ {
        self.map
            .get(&pos.into())
            .map(|v| v.iter().copied())
            .unwrap_or_default()
    }

    pub(super) fn insert(&mut self, pos: impl Into<ChunkPos>, client: Entity) -> bool {
        match self.map.entry(pos.into()) {
            Entry::Occupied(oe) => {
                let v = oe.into_mut();

                if !v.contains(&client) {
                    v.push(client);
                    true
                } else {
                    false
                }
            }
            Entry::Vacant(ve) => {
                ve.insert(vec![client]);
                true
            }
        }
    }

    pub(super) fn remove(&mut self, pos: impl Into<ChunkPos>, client: Entity) -> bool {
        match self.map.entry(pos.into()) {
            Entry::Occupied(mut oe) => {
                let v = oe.get_mut();

                if let Some(idx) = v.iter().copied().position(|e| e == client) {
                    v.swap_remove(idx);

                    if v.is_empty() {
                        oe.remove();
                    }

                    true
                } else {
                    false
                }
            }
            Entry::Vacant(_) => false,
        }
    }
}
