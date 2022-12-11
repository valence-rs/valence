use std::collections::hash_map::Entry;
use std::collections::BTreeSet;

use crate::chunk::ChunkPos;
use crate::config::Config;
use crate::entity::{Entities, EntityId};
use crate::packet::PacketWriter;
use crate::world::Worlds;

pub struct PartitionCell {
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
    /// A cache of packets needed to update all the `entities` in this chunk.
    cached_update_packets: Vec<u8>,
}

impl PartitionCell {
    pub(super) fn new() -> Self {
        Self {
            entities: BTreeSet::new(),
            incoming: vec![],
            outgoing: vec![],
            cached_update_packets: vec![],
        }
    }

    pub fn entities(&self) -> impl ExactSizeIterator<Item = EntityId> + '_ {
        self.entities.iter().cloned()
    }

    pub fn incoming(&self) -> &[(EntityId, Option<ChunkPos>)] {
        &self.incoming
    }

    pub fn outgoing(&self) -> &[(EntityId, Option<ChunkPos>)] {
        &self.outgoing
    }

    pub fn cached_update_packets(&self) -> &[u8] {
        &self.cached_update_packets
    }

    pub(super) fn clear_incoming_outgoing(&mut self) {
        self.incoming.clear();
        self.outgoing.clear();
    }
}

/// Prepares the entity partitions in all worlds for the client update
/// procedure.
pub fn update_entity_partition<C: Config>(
    entities: &mut Entities<C>,
    worlds: &mut Worlds<C>,
    compression_threshold: Option<u32>,
) {
    for (entity_id, entity) in entities.iter() {
        let pos = ChunkPos::at(entity.position().x, entity.position().z);
        let old_pos = ChunkPos::at(entity.old_position().x, entity.old_position().z);

        let world = entity.world();
        let old_world = entity.old_world();

        if entity.deleted() {
            // Entity was deleted. Remove it from the chunk it was in, if it was in a chunk
            // at all.
            if let Some(old_world) = worlds.get_mut(old_world) {
                if let Some(old_cell) = old_world.chunks.cell_mut(old_pos) {
                    if old_cell.entities.remove(&entity_id) {
                        old_cell.outgoing.push((entity_id, None));
                    }
                }
            }
        } else if old_world != world {
            // TODO: skip marker entity.

            // Entity changed the world it is in. Remove it from old chunk and
            // insert it in the new chunk.
            if let Some(old_world) = worlds.get_mut(old_world) {
                if let Some(old_cell) = old_world.chunks.cell_mut(old_pos) {
                    if old_cell.entities.remove(&entity_id) {
                        old_cell.outgoing.push((entity_id, None));
                    }
                }
            }

            if let Some(world) = worlds.get_mut(world) {
                match world.chunks.chunks.entry(pos) {
                    Entry::Occupied(oe) => {
                        let cell = &mut oe.into_mut().1;
                        if cell.entities.insert(entity_id) {
                            cell.incoming.push((entity_id, None));
                        }
                    }
                    Entry::Vacant(ve) => {
                        let cell = PartitionCell {
                            entities: BTreeSet::from([entity_id]),
                            incoming: vec![(entity_id, None)],
                            outgoing: vec![],
                            cached_update_packets: vec![],
                        };

                        ve.insert((None, cell));
                    }
                }
            }
        } else if pos != old_pos {
            // TODO: skip marker entity.

            // Entity changed its chunk position without changing worlds. Remove
            // it from old chunk and insert it in new chunk.
            if let Some(world) = worlds.get_mut(world) {
                if let Some(old_cell) = world.chunks.cell_mut(old_pos) {
                    if old_cell.entities.remove(&entity_id) {
                        old_cell.outgoing.push((entity_id, Some(pos)));
                    }
                }

                match world.chunks.chunks.entry(pos) {
                    Entry::Occupied(oe) => {
                        let cell = &mut oe.into_mut().1;
                        if cell.entities.insert(entity_id) {
                            cell.incoming.push((entity_id, Some(old_pos)));
                        }
                    }
                    Entry::Vacant(ve) => {
                        let cell = PartitionCell {
                            entities: BTreeSet::from([entity_id]),
                            incoming: vec![(entity_id, Some(old_pos))],
                            outgoing: vec![],
                            cached_update_packets: vec![],
                        };

                        ve.insert((None, cell));
                    }
                }
            }
        } else {
            // The entity didn't change its chunk position so there is nothing
            // we need to do.
        }
    }

    // Cache the entity update packets.
    let mut scratch = vec![];
    let mut compression_scratch = vec![];

    for (_, world) in worlds.iter_mut() {
        for cell in world.chunks.cells_mut() {
            cell.cached_update_packets.clear();

            for &id in &cell.entities {
                let start = cell.cached_update_packets.len();

                let writer = PacketWriter::new(
                    &mut cell.cached_update_packets,
                    compression_threshold,
                    &mut compression_scratch,
                );

                let entity = &mut entities[id];

                entity
                    .write_update_packets(writer, id, &mut scratch)
                    .unwrap();

                let end = cell.cached_update_packets.len();
                entity.self_update_range = start..end;
            }
        }
    }
}
