use std::collections::hash_map::Entry;
use std::collections::{BTreeSet, HashMap};

use crate::chunk::ChunkPos;
use crate::config::Config;
use crate::entity::{Entities, EntityId};
use crate::world::Worlds;

pub struct EntityPartition {
    cells: HashMap<ChunkPos, ChunkCell>,
}

pub struct ChunkCell {
    /// Entities in this cell.
    /// Invariant: After [`update_entity_partition`] is called, contains only
    /// valid IDs and non-deleted entities with positions inside this cell.
    entities: BTreeSet<EntityId>,
    /// Entities that have entered the chunk this tick, paired with the cell
    /// position in this world they came from.
    incoming: Vec<(EntityId, Option<ChunkPos>)>,
    /// Entities that have left the chunk this tick, paired with the cell
    /// position in this world they arrived at.
    outgoing: Vec<(EntityId, Option<ChunkPos>)>,
}

impl EntityPartition {
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
        }
    }

    pub fn get(&self, pos: ChunkPos) -> Option<&ChunkCell> {
        self.cells.get(&pos)
    }

    pub fn clear_for_next_tick(&mut self) {
        self.cells.retain(|_, cell| {
            if cell.entities.is_empty() {
                false
            } else {
                cell.incoming.clear();
                cell.outgoing.clear();
                true
            }
        });
    }
}

impl ChunkCell {
    pub fn entities(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.entities.iter().cloned()
    }

    pub fn incoming(&self) -> &[(EntityId, Option<ChunkPos>)] {
        &self.incoming
    }

    pub fn outgoing(&self) -> &[(EntityId, Option<ChunkPos>)] {
        &self.outgoing
    }
}

/// Prepares the entity partitions in all worlds for the client update
/// procedure.
pub fn update_entity_partition<C: Config>(entities: &Entities<C>, worlds: &mut Worlds<C>) {
    for (entity_id, entity) in entities.iter() {
        let pos = ChunkPos::at(entity.position().x, entity.position().z);
        let old_pos = ChunkPos::at(entity.old_position().x, entity.old_position().z);

        let world = entity.world();
        let old_world = entity.old_world();

        if entity.deleted() {
            // Entity was deleted. Remove it from the chunk it was in, if it was in a chunk
            // at all.
            if let Some(old_world) = worlds.get_mut(old_world) {
                if let Some(old_cell) = old_world.entity_partition.cells.get_mut(&old_pos) {
                    if old_cell.entities.remove(&entity_id) {
                        old_cell.outgoing.push((entity_id, None));
                    }
                }
            }
        } else if old_world != world {
            // Entity changed the world it is in. Remove it from old chunk and
            // insert it in the new chunk.
            if let Some(old_world) = worlds.get_mut(old_world) {
                if let Some(old_cell) = old_world.entity_partition.cells.get_mut(&old_pos) {
                    if old_cell.entities.remove(&entity_id) {
                        old_cell.outgoing.push((entity_id, None));
                    }
                }
            }

            if let Some(world) = worlds.get_mut(world) {
                match world.entity_partition.cells.entry(pos) {
                    Entry::Occupied(oe) => {
                        let cell = oe.into_mut();
                        if cell.entities.insert(entity_id) {
                            cell.incoming.push((entity_id, None));
                        }
                    }
                    Entry::Vacant(ve) => {
                        ve.insert(ChunkCell {
                            entities: BTreeSet::from([entity_id]),
                            incoming: vec![(entity_id, None)],
                            outgoing: Vec::new(),
                        });
                    }
                }
            }
        } else if pos != old_pos {
            // Entity changed its chunk position without changing worlds. Remove
            // it from old chunk and insert it in new chunk.
            if let Some(world) = worlds.get_mut(world) {
                if let Some(old_cell) = world.entity_partition.cells.get_mut(&old_pos) {
                    if old_cell.entities.remove(&entity_id) {
                        old_cell.outgoing.push((entity_id, Some(pos)));
                    }
                }

                match world.entity_partition.cells.entry(pos) {
                    Entry::Occupied(oe) => {
                        let cell = oe.into_mut();
                        if cell.entities.insert(entity_id) {
                            cell.incoming.push((entity_id, Some(old_pos)));
                        }
                    }
                    Entry::Vacant(ve) => {
                        ve.insert(ChunkCell {
                            entities: BTreeSet::from([entity_id]),
                            incoming: vec![(entity_id, Some(old_pos))],
                            outgoing: Vec::new(),
                        });
                    }
                }
            }
        } else {
            // The entity didn't change its chunk position so there is nothing
            // we need to do.
        }
    }
}

/*
impl EntitiesInChunk {
    pub(super) fn new() -> Self {
        Self {
            entities: BTreeSet::new(),
            incoming: Vec::new(),
            outgoing: Vec::new(),
        }
    }

    pub(super) fn optimize(&mut self) {
        self.incoming.shrink_to_fit();
        self.outgoing.shrink_to_fit();
    }

    pub(super) fn clear_incoming_outgoing(&mut self) {
        self.incoming.clear();
        self.outgoing.clear();
    }

    pub fn entities(&self) -> impl Iterator<Item = EntityId> + '_ {
        self.entities.iter().cloned()
    }

    pub fn incoming(&self) -> &[(EntityId, Option<ChunkPos>)] {
        &self.incoming
    }

    pub fn outgoing(&self) -> &[(EntityId, Option<ChunkPos>)] {
        &self.outgoing
    }
}

pub fn update_entity_partition<C: Config>(entities: &mut Entities<C>, worlds: &mut Worlds<C>) {
    for (entity_id, entity) in entities.iter_mut() {
        let pos = ChunkPos::at(entity.position().x, entity.position().z);
        let old_pos = ChunkPos::at(entity.old_position().x, entity.old_position().z);

        let world = entity.world();
        let old_world = entity.old_world();

        if entity.deleted() {
            // Entity was deleted. Remove it from the chunk it was in.
            if let Some(old_world) = worlds.get_mut(entity.old_world()) {
                if let Some(old_chunk) = old_world.chunks.get_mut(old_pos) {
                    if old_chunk.entities.entities.remove(&entity_id) {
                        old_chunk.entities.outgoing.push((entity_id, None));
                    }
                }
            }
        } else if old_world != world {
            // Entity changed the world it is in. Remove it from old chunk and insert it in
            // the new chunk.
            if let Some(old_world) = worlds.get_mut(old_world) {
                if let Some(old_chunk) = old_world.chunks.get_mut(old_pos) {
                    if old_chunk.entities.entities.remove(&entity_id) {
                        old_chunk.entities.outgoing.push((entity_id, None));
                    }
                }
            }

            if let Some(world) = worlds.get_mut(world) {
                if let Some(chunk) = world.chunks.get_mut(pos) {
                    if chunk.entities.entities.insert(entity_id) {
                        chunk.entities.incoming.push((entity_id, None));
                    }
                }
            }
        } else if pos != old_pos {
            // Entity changed its chunk position. Remove it from old chunk and insert it in
            // new chunk.
            if let Some(world) = worlds.get_mut(world) {
                let old_chunk_loaded = world.chunks.get(old_pos).is_some();
                let chunk_loaded = world.chunks.get(pos).is_some();

                if let Some(old_chunk) = world.chunks.get_mut(old_pos) {
                    if old_chunk.entities.entities.remove(&entity_id) {
                        old_chunk
                            .entities
                            .outgoing
                            .push((entity_id, chunk_loaded.then_some(pos)));
                    }
                }

                if let Some(chunk) = world.chunks.get_mut(pos) {
                    if chunk.entities.entities.insert(entity_id) {
                        chunk
                            .entities
                            .incoming
                            .push((entity_id, old_chunk_loaded.then_some(old_pos)));
                    }
                }
            }
        } else {
            // The entity didn't change its chunk position at all. However, we still need to
            // try adding it to its current chunk because of one situation; A chunk may have
            // been created at the position the entity was already located at.

            // TODO: we can avoid some of this work by maintaining the set of new chunks.

            if let Some(world) = worlds.get_mut(world) {
                if let Some(chunk) = world.chunks.get_mut(pos) {
                    if chunk.entities.entities.insert(entity_id) {
                        chunk.entities.incoming.push((entity_id, None));
                    }
                }
            }
        }
    }
}
*/
