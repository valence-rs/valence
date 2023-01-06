use std::collections::hash_map::Entry;
use std::collections::BTreeSet;
use std::iter::FusedIterator;

use bevy_ecs::prelude::*;
use num::integer::div_ceil;
use rustc_hash::FxHashMap;
use valence_protocol::{BlockPos, LengthPrefixedArray};

use crate::chunk_pos::ChunkPos;
use crate::dimension::DimensionId;
use crate::entity::McEntity;
use crate::instance::chunk::Chunk;
use crate::packet::PacketWriter;
use crate::server::SharedServer;
use crate::Despawned;

pub mod chunk;
mod paletted_container;

/// To create a new instance, see [`SharedServer::new_instance`].
#[derive(Component)]
pub struct Instance {
    partition: FxHashMap<ChunkPos, PartitionCell>,
    dimension: DimensionId,
    section_count: usize,
    min_y: i32,
    compression_threshold: Option<u32>,
    filler_sky_light_mask: Box<[u64]>,
    /// Sending filler light data causes the vanilla client to lag
    /// less. Hopefully we can remove this in the future.
    filler_sky_light_arrays: Box<[LengthPrefixedArray<u8, 2048>]>,
    /// Packet data to send to all clients in this instance at the end of the
    /// tick.
    packet_buf: Vec<u8>,
}

pub(crate) struct PartitionCell {
    /// The chunk in this cell.
    chunk: Option<Chunk<true>>,
    /// If the chunk went from `Some` to `None` this tick.
    chunk_removed: bool,
    /// Minecraft entities in this cell.
    entities: BTreeSet<Entity>,
    /// Entities that have entered the cell this tick, paired with the cell
    /// position in this instance they came from.
    incoming: Vec<(Entity, Option<ChunkPos>)>,
    /// Entities that have left the cell this tick, paired with the cell
    /// position in this instance they arrived at.
    outgoing: Vec<(Entity, Option<ChunkPos>)>,
    /// A cache of packets to send to all clients that are in view of this cell
    /// at the end of the tick.
    packet_buf: Vec<u8>,
}

impl Instance {
    pub(crate) fn new(dimension: DimensionId, shared: &SharedServer) -> Self {
        let dim = shared.dimension(dimension);

        let light_section_count = (dim.height / 16 + 2) as usize;

        let mut sky_light_mask = vec![0; div_ceil(light_section_count, 16)];

        for i in 0..light_section_count {
            sky_light_mask[i / 64] |= 1 << (i % 64);
        }

        Self {
            partition: FxHashMap::default(),
            dimension,
            section_count: (dim.height / 16) as usize,
            min_y: dim.min_y,
            compression_threshold: shared.compression_threshold(),
            filler_sky_light_mask: sky_light_mask.into(),
            filler_sky_light_arrays: vec![LengthPrefixedArray([0xff; 2048]); light_section_count]
                .into(),
            packet_buf: vec![],
        }
    }

    pub fn dimension(&self) -> DimensionId {
        self.dimension
    }

    pub fn section_count(&self) -> usize {
        self.section_count
    }

    pub fn insert_chunk(&mut self, pos: ChunkPos, chunk: Chunk) -> Option<Chunk> {
        match self.partition.entry(pos) {
            Entry::Occupied(oe) => {
                let cell = oe.into_mut();
                cell.chunk
                    .replace(chunk.into_loaded())
                    .map(|c| c.into_unloaded())
            }
            Entry::Vacant(ve) => {
                ve.insert(PartitionCell {
                    chunk: Some(chunk.into_loaded()),
                    chunk_removed: false,
                    entities: BTreeSet::new(),
                    incoming: vec![],
                    outgoing: vec![],
                    packet_buf: vec![],
                });

                None
            }
        }
    }

    pub fn remove_chunk(&mut self, pos: ChunkPos) -> Option<Chunk> {
        self.partition.get_mut(&pos).and_then(|p| {
            let chunk = p.chunk.take().map(|c| c.into_unloaded());
            p.chunk_removed = chunk.is_some();
            chunk
        })
    }

    pub fn chunk(&self, pos: ChunkPos) -> Option<&Chunk<true>> {
        self.partition.get(&pos).and_then(|p| p.chunk.as_ref())
    }

    pub fn chunk_mut(&mut self, pos: ChunkPos) -> Option<&mut Chunk<true>> {
        self.partition.get_mut(&pos).and_then(|p| p.chunk.as_mut())
    }

    pub fn chunks(&self) -> impl FusedIterator<Item = (ChunkPos, &Chunk<true>)> + Clone + '_ {
        self.partition
            .iter()
            .flat_map(|(&pos, par)| par.chunk.as_ref().map(|c| (pos, c)))
    }

    pub fn chunks_mut(&mut self) -> impl FusedIterator<Item = (ChunkPos, &mut Chunk<true>)> + '_ {
        self.partition
            .iter_mut()
            .flat_map(|(&pos, par)| par.chunk.as_mut().map(|c| (pos, c)))
    }

    pub fn optimize(&mut self) {
        for cell in self.partition.values_mut() {
            if let Some(chunk) = &mut cell.chunk {
                chunk.optimize();
            }
            cell.incoming.shrink_to_fit();
            cell.outgoing.shrink_to_fit();
        }

        self.partition.shrink_to_fit();
    }
}

pub(crate) fn update_instances_pre_client(
    mut instances: Query<&mut Instance>,
    mut entities: Query<(Entity, &mut McEntity, Option<&Despawned>), Changed<McEntity>>,
) {
    // Update the partitions that entities are in.
    for (entity_id, entity, despawned) in &mut entities {
        let pos = ChunkPos::at(entity.position().x, entity.position().z);
        let old_pos = ChunkPos::at(entity.old_position().x, entity.old_position().z);

        let instance = entity.instance();
        let old_instance = entity.old_instance();

        if despawned.is_some() {
            // Entity was despawned. Remove it from the cell it was in, if it
            // was in a cell at all.
            if let Ok(mut old_instance) = instances.get_mut(old_instance) {
                if let Some(old_cell) = old_instance.partition.get_mut(&old_pos) {
                    if old_cell.entities.remove(&entity_id) {
                        old_cell.outgoing.push((entity_id, None));
                    }
                }
            }
        } else if old_instance != instance {
            // TODO: skip marker entity?
            // Entity changed the instance it is in. Remove it from the cell in
            // the old instance and insert it into the cell in the new instance.

            if let Ok(mut old_instance) = instances.get_mut(old_instance) {
                if let Some(old_cell) = old_instance.partition.get_mut(&old_pos) {
                    if old_cell.entities.remove(&entity_id) {
                        old_cell.outgoing.push((entity_id, None));
                    }
                }
            }

            if let Ok(mut instance) = instances.get_mut(instance) {
                match instance.partition.entry(pos) {
                    Entry::Occupied(oe) => {
                        let cell = oe.into_mut();
                        if cell.entities.insert(entity_id) {
                            cell.incoming.push((entity_id, None));
                        }
                    }
                    Entry::Vacant(ve) => {
                        ve.insert(PartitionCell {
                            chunk: None,
                            chunk_removed: false,
                            entities: BTreeSet::from([entity_id]),
                            incoming: vec![(entity_id, None)],
                            outgoing: vec![],
                            packet_buf: vec![],
                        });
                    }
                }
            }
        } else if pos != old_pos {
            // TODO: skip marker entity?
            // Entity changed its chunk position without changing instances.
            // Remove it from the old cell and insert it into the new cell.

            if let Ok(mut instance) = instances.get_mut(instance) {
                if let Some(old_cell) = instance.partition.get_mut(&old_pos) {
                    if old_cell.entities.remove(&entity_id) {
                        old_cell.outgoing.push((entity_id, Some(pos)));
                    }
                }

                match instance.partition.entry(pos) {
                    Entry::Occupied(oe) => {
                        let cell = oe.into_mut();
                        if cell.entities.insert(entity_id) {
                            cell.incoming.push((entity_id, Some(old_pos)));
                        }
                    }
                    Entry::Vacant(ve) => {
                        ve.insert(PartitionCell {
                            chunk: None,
                            chunk_removed: false,
                            entities: BTreeSet::from([entity_id]),
                            incoming: vec![(entity_id, Some(old_pos))],
                            outgoing: vec![],
                            packet_buf: vec![],
                        });
                    }
                }
            }
        } else {
            // The entity didn't change its chunk position so there is nothing
            // to do.
        }
    }

    // Cache the entity update packets and chunk update packets.
    let mut scratch = vec![];
    let mut compression_scratch = vec![];

    for mut instance in &mut instances {
        let compression_threshold = instance.compression_threshold;
        let min_y = instance.min_y;

        for (&pos, mut cell) in instance.partition.iter_mut() {
            if let Some(chunk) = &mut cell.chunk {
                chunk.update_pre_client(
                    pos,
                    min_y,
                    &mut cell.packet_buf,
                    compression_threshold,
                    &mut scratch,
                );
            }

            for &entity_id in &cell.entities {
                let start = cell.packet_buf.len();

                let writer = PacketWriter::new(
                    &mut cell.packet_buf,
                    compression_threshold,
                    &mut compression_scratch,
                );

                let Ok((_, mut entity, _)) = entities.get_mut(entity_id) else {
                    continue
                };

                entity.write_update_packets(writer, &mut scratch).unwrap();

                let end = cell.packet_buf.len();
                entity.self_update_range = start..end;
            }
        }
    }
}

pub(crate) fn update_instances_post_client(mut instances: Query<&mut Instance>) {
    for mut instance in &mut instances {
        instance.partition.retain(|_, cell| {
            cell.chunk_removed = false;
            cell.incoming.clear();
            cell.outgoing.clear();

            if let Some(chunk) = &mut cell.chunk {
                chunk.update_post_client();
            }

            cell.chunk.is_some() || cell.entities.len() > 0
        });
    }
}
