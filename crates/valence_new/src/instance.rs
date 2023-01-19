use std::collections::BTreeSet;
use std::hash::BuildHasherDefault;
use std::iter::FusedIterator;

use bevy_ecs::prelude::*;
pub use chunk_entry::*;
use hashbrown::hash_map::Entry;
use hashbrown::HashMap;
use num::integer::div_ceil;
use rustc_hash::FxHasher;
use valence_protocol::packets::s2c::play::UnloadChunk;
use valence_protocol::LengthPrefixedArray;

use crate::chunk_pos::ChunkPos;
use crate::client::Client;
use crate::dimension::DimensionId;
use crate::entity::McEntity;
pub use crate::instance::chunk::Chunk;
use crate::packet::{PacketWriter, WritePacket};
use crate::server::{Server, SharedServer};
use crate::Despawned;

mod chunk;
mod chunk_entry;
mod paletted_container;

/// To create a new instance, see [`SharedServer::new_instance`].
#[derive(Component)]
pub struct Instance {
    pub(crate) partition: HashMap<ChunkPos, PartitionCell, BuildHasherDefault<FxHasher>>,
    pub(crate) info: InstanceInfo,
    /// Packet data to send to all clients in this instance at the end of the
    /// tick.
    pub(crate) packet_buf: Vec<u8>,
}

pub(crate) struct InstanceInfo {
    dimension: DimensionId,
    section_count: usize,
    min_y: i32,
    biome_registry_len: usize,
    compression_threshold: Option<u32>,
    filler_sky_light_mask: Box<[u64]>,
    /// Sending filler light data causes the vanilla client to lag
    /// less. Hopefully we can remove this in the future.
    filler_sky_light_arrays: Box<[LengthPrefixedArray<u8, 2048>]>,
}

#[derive(Debug)]
pub(crate) struct PartitionCell {
    /// The chunk in this cell.
    pub(crate) chunk: Option<Chunk<true>>,
    /// If `chunk` went from `Some` to `None` this tick.
    chunk_removed: bool,
    /// All of the client entities in view of this cell.
    pub(crate) viewers: BTreeSet<Entity>,
    /// Minecraft entities in this cell.
    pub(crate) entities: BTreeSet<Entity>,
    /// A cache of packets to send to all clients that are in view of this cell
    /// at the end of the tick.
    pub(crate) packet_buf: Vec<u8>,
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
            partition: HashMap::default(),
            info: InstanceInfo {
                dimension,
                section_count: (dim.height / 16) as usize,
                min_y: dim.min_y,
                biome_registry_len: shared.biomes().len(),
                compression_threshold: shared.compression_threshold(),
                filler_sky_light_mask: sky_light_mask.into(),
                filler_sky_light_arrays: vec![
                    LengthPrefixedArray([0xff; 2048]);
                    light_section_count
                ]
                .into(),
            },
            packet_buf: vec![],
        }
    }

    pub fn dimension(&self) -> DimensionId {
        self.info.dimension
    }

    pub fn section_count(&self) -> usize {
        self.info.section_count
    }

    pub fn chunk(&self, pos: impl Into<ChunkPos>) -> Option<&Chunk<true>> {
        self.partition
            .get(&pos.into())
            .and_then(|p| p.chunk.as_ref())
    }

    pub fn chunk_mut(&mut self, pos: impl Into<ChunkPos>) -> Option<&mut Chunk<true>> {
        self.partition
            .get_mut(&pos.into())
            .and_then(|p| p.chunk.as_mut())
    }

    pub fn insert_chunk(&mut self, pos: impl Into<ChunkPos>, chunk: Chunk) -> Option<Chunk> {
        match self.chunk_entry(pos) {
            ChunkEntry::Occupied(mut oe) => Some(oe.insert(chunk)),
            ChunkEntry::Vacant(ve) => {
                ve.insert(chunk);
                None
            }
        }
    }

    pub fn remove_chunk(&mut self, pos: impl Into<ChunkPos>) -> Option<Chunk> {
        match self.chunk_entry(pos) {
            ChunkEntry::Occupied(oe) => Some(oe.remove()),
            ChunkEntry::Vacant(_) => None,
        }
    }

    pub fn clear_chunks(&mut self) {
        for cell in &mut self.partition.values_mut() {
            if cell.chunk.take().is_some() {
                cell.chunk_removed = true;
            }
        }
    }

    pub fn chunk_entry(&mut self, pos: impl Into<ChunkPos>) -> ChunkEntry {
        ChunkEntry::new(self.info.section_count, self.partition.entry(pos.into()))
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
        for (_, chunk) in self.chunks_mut() {
            chunk.optimize();
        }

        self.partition.shrink_to_fit();
        self.packet_buf.shrink_to_fit();
    }
}

pub(crate) fn update_instances_pre_client(
    mut instances: Query<&mut Instance>,
    // TODO: Check if adding Changed<McEntity> filter would break things.
    mut entities: Query<(Entity, &McEntity, Option<&Despawned>)>,
    mut clients: Query<&mut Client>,
    server: Res<Server>,
) {
    let mut scratch = vec![];
    let mut compression_scratch = vec![];
    let mut scratch_2 = vec![];

    // Loop over every entity and change their partition cells if their location
    // changed.
    for (entity_id, entity, despawned) in &mut entities {
        let old_pos = ChunkPos::at(entity.old_position().x, entity.old_position().z);
        let pos = ChunkPos::at(entity.position().x, entity.position().z);

        if despawned.is_some() {
            if let Ok(mut old_instance) = instances.get_mut(entity.old_instance()) {
                if let Some(old_cell) = old_instance.partition.get_mut(&old_pos) {
                    assert!(old_cell.entities.remove(&entity_id));

                    for &client_id in &old_cell.viewers {
                        if let Ok(mut client) = clients.get_mut(client_id) {
                            // TODO: check that entity is not the self-entity.
                            client.despawn_entity(entity.protocol_id());
                        }
                    }
                }
            }
        } else if entity.old_instance() != entity.instance() {
            if let Ok(mut old_instance) = instances.get_mut(entity.old_instance()) {
                if let Some(old_cell) = old_instance.partition.get_mut(&old_pos) {
                    assert!(old_cell.entities.remove(&entity_id));

                    for &client_id in &old_cell.viewers {
                        if let Ok(mut client) = clients.get_mut(client_id) {
                            // TODO: check that entity is not the self-entity.
                            client.despawn_entity(entity.protocol_id());
                        }
                    }
                }
            }

            if let Ok(mut instance) = instances.get_mut(entity.instance()) {
                match instance.partition.entry(pos) {
                    Entry::Occupied(oe) => {
                        let cell = oe.into_mut();
                        assert!(cell.entities.insert(entity_id));

                        if !cell.viewers.is_empty() {
                            scratch.clear();
                            let writer = PacketWriter::new(
                                &mut scratch,
                                server.compression_threshold(),
                                &mut compression_scratch,
                            );

                            // Write with the old position so that the entity will be in the correct
                            // position if the later entity update packets include a relative
                            // movement.
                            entity
                                .write_init_packets(writer, entity.old_position(), &mut scratch_2)
                                .unwrap();

                            // TODO: write it to the cell's packet_buf instead?
                            for &client_id in &cell.viewers {
                                if let Ok(mut client) = clients.get_mut(client_id) {
                                    // TODO: check that entity is not the self-entity.
                                    client.write_packet_bytes(&scratch);
                                }
                            }
                        }
                    }
                    Entry::Vacant(ve) => {
                        ve.insert(PartitionCell {
                            chunk: None,
                            chunk_removed: false,
                            viewers: BTreeSet::new(),
                            entities: BTreeSet::from([entity_id]),
                            packet_buf: vec![],
                        });
                    }
                }
            }
        } else if old_pos != pos {
            if let Ok(mut instance) = instances.get_mut(entity.instance()) {
                if let Entry::Vacant(ve) = instance.partition.entry(pos) {
                    ve.insert(PartitionCell {
                        chunk: None,
                        chunk_removed: false,
                        viewers: BTreeSet::new(),
                        // Code below will add the entity to this set.
                        entities: BTreeSet::new(),
                        packet_buf: vec![],
                    });
                }

                // Old and new cells should exist so this `get_many_mut` shouldn't fail.
                let [old_cell, cell] = instance.partition.get_many_mut([&old_pos, &pos]).unwrap();

                assert!(old_cell.entities.remove(&entity_id));
                assert!(cell.entities.insert(entity_id));

                for &client_id in old_cell.viewers.difference(&cell.viewers) {
                    if let Ok(mut client) = clients.get_mut(client_id) {
                        // The entity exited the view of this client.
                        // TODO: check that entity is not the self-entity.
                        client.despawn_entity(entity.protocol_id());
                    }
                }

                scratch.clear();
                for &client_id in cell.viewers.difference(&old_cell.viewers) {
                    if let Ok(mut client) = clients.get_mut(client_id) {
                        // The entity entered the view of this client.

                        // TODO: `continue` if entity is the self-entity.

                        if scratch.is_empty() {
                            let writer = PacketWriter::new(
                                &mut scratch,
                                server.compression_threshold(),
                                &mut compression_scratch,
                            );

                            entity
                                .write_init_packets(writer, entity.old_position(), &mut scratch_2)
                                .unwrap();
                        }

                        client.write_packet_bytes(&scratch);
                    }
                }
            }
        }
    }

    // TODO: move this to separate system?

    // Write the entity and chunk update packets to clients. Also write any data
    // added to each cell's packet buffer.
    for instance in &mut instances {
        // To allow for splitting borrows.
        let instance = instance.into_inner();

        for (&pos, cell) in &mut instance.partition {
            if !cell.viewers.is_empty() {
                let mut writer = PacketWriter::new(
                    &mut cell.packet_buf,
                    server.compression_threshold(),
                    &mut compression_scratch,
                );

                for &entity_id in &cell.entities {
                    if let Ok((_, entity, _)) = entities.get(entity_id) {
                        entity
                            .write_update_packets(&mut writer, &mut scratch_2)
                            .unwrap();
                    }
                }

                if let Some(chunk) = &mut cell.chunk {
                    chunk
                        .write_update_packets(writer, &mut scratch_2, pos, &instance.info)
                        .unwrap();
                } else if cell.chunk_removed {
                    writer
                        .write_packet(&UnloadChunk {
                            chunk_x: pos.x,
                            chunk_z: pos.z,
                        })
                        .unwrap();
                }

                if !cell.packet_buf.is_empty() {
                    for &client_id in &cell.viewers {
                        if let Ok(mut client) = clients.get_mut(client_id) {
                            client.write_packet_bytes(&cell.packet_buf);
                        }
                    }
                }
            }
        }
    }
}

pub(crate) fn update_instances_post_client(mut instances: Query<&mut Instance>) {
    for mut instance in &mut instances {
        instance.partition.retain(|_, cell| {
            cell.packet_buf.clear();
            cell.chunk_removed = false;

            if let Some(chunk) = &mut cell.chunk {
                chunk.update_post_client();
            }

            cell.chunk.is_some() || cell.entities.len() > 0
        });

        instance.packet_buf.clear();
    }
}
