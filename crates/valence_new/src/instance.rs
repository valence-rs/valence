use std::collections::hash_map::Entry;
use std::collections::BTreeSet;
use std::iter::FusedIterator;

use bevy_ecs::prelude::*;
pub use chunk_entry::*;
use num::integer::div_ceil;
use rustc_hash::FxHashMap;
use valence_protocol::block::BlockState;
use valence_protocol::{BlockPos, EncodePacket, LengthPrefixedArray};

use crate::view::ChunkPos;
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
    pub(crate) partition: FxHashMap<ChunkPos, PartitionCell>,
    pub(crate) info: InstanceInfo,
    /// Packet data to send to all clients in this instance at the end of the
    /// tick.
    pub(crate) packet_buf: Vec<u8>,
    /// Scratch space for writing packets.
    scratch: Vec<u8>,
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
    pub(crate) chunk_removed: bool,
    /// Minecraft entities in this cell.
    pub(crate) entities: BTreeSet<Entity>,
    /// Minecraft entities that have entered the chunk this tick, paired with
    /// the cell position in this instance they came from.
    pub(crate) incoming: Vec<(Entity, Option<ChunkPos>)>,
    /// Minecraft entities that have left the chunk this tick, paired with the
    /// cell position in this world they arrived at.
    pub(crate) outgoing: Vec<(Entity, Option<ChunkPos>)>,
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
            partition: FxHashMap::default(),
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
            scratch: vec![],
        }
    }

    /// Writes a packet into the global packet buffer of this instance. All
    /// clients in the instance will receive the packet.
    ///
    /// This is more efficient than sending the packet to each client
    /// individually.
    pub fn write_packet<P>(&mut self, pkt: &P)
    where
        P: EncodePacket + ?Sized,
    {
        PacketWriter::new(
            &mut self.packet_buf,
            self.info.compression_threshold,
            &mut self.scratch,
        )
        .write_packet(pkt);
    }

    /// Writes a packet to all clients in view of `pos` in this instance. Has no
    /// effect if there is no chunk at `pos`.
    ///
    /// This is more efficient than sending the packet to each client
    /// individually.
    pub fn write_packet_at<P>(&mut self, pkt: &P, pos: impl Into<ChunkPos>)
    where
        P: EncodePacket + ?Sized,
    {
        let pos = pos.into();
        if let Some(cell) = self.partition.get_mut(&pos) {
            if cell.chunk.is_some() {
                PacketWriter::new(
                    &mut cell.packet_buf,
                    self.info.compression_threshold,
                    &mut self.scratch,
                )
                .write_packet(pkt);
            }
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

    /// Gets the block state at an absolute block position in world space.
    ///
    /// If the position is not inside of a chunk, then [`BlockState::AIR`] is
    /// returned.
    pub fn block_state(&self, pos: impl Into<BlockPos>) -> BlockState {
        let pos = pos.into();

        let Some(y) = pos.y.checked_sub(self.info.min_y).and_then(|y| y.try_into().ok()) else {
            return BlockState::AIR;
        };

        if y >= self.info.section_count * 16 {
            return BlockState::AIR;
        }

        let Some(chunk) = self.chunk(ChunkPos::from_block_pos(pos)) else {
            return BlockState::AIR;
        };

        chunk.block_state(
            pos.x.rem_euclid(16) as usize,
            y,
            pos.z.rem_euclid(16) as usize,
        )
    }

    /// Sets the block state at an absolute block position in world space. The
    /// previous block state at the position is returned.
    ///
    /// If the position is not within a loaded chunk or otherwise out of bounds,
    /// then [`BlockState::AIR`] is returned with no effect.
    pub fn set_block_state(&mut self, pos: impl Into<BlockPos>, block: BlockState) -> BlockState {
        let pos = pos.into();

        let Some(y) = pos.y.checked_sub(self.info.min_y).and_then(|y| y.try_into().ok()) else {
            return BlockState::AIR;
        };

        if y >= self.info.section_count * 16 {
            return BlockState::AIR;
        }

        let Some(chunk) = self.chunk_mut(ChunkPos::from_block_pos(pos)) else {
            return BlockState::AIR;
        };

        chunk.set_block_state(
            pos.x.rem_euclid(16) as usize,
            y,
            pos.z.rem_euclid(16) as usize,
            block,
        )
    }
}

pub(crate) fn update_instances_pre_client(
    mut instances: Query<&mut Instance>,
    mut entities: Query<(Entity, &mut McEntity, Option<&Despawned>)>,
    server: Res<Server>,
) {
    for (entity_id, entity, despawned) in &entities {
        let pos = ChunkPos::at(entity.position().x, entity.position().z);
        let old_pos = ChunkPos::at(entity.old_position().x, entity.old_position().z);

        let instance = entity.instance();
        let old_instance = entity.old_instance();

        if despawned.is_some() {
            // Entity was deleted. Remove it from the chunk it was in, if it was in a chunk
            // at all.
            if let Ok(mut old_instance) = instances.get_mut(old_instance) {
                if let Some(old_cell) = old_instance.partition.get_mut(&old_pos) {
                    if old_cell.entities.remove(&entity_id) {
                        old_cell.outgoing.push((entity_id, None));
                    }
                }
            }
        } else if old_instance != instance {
            // Entity changed the instance it is in. Remove it from old cell and
            // insert it in the new cell.

            // TODO: skip marker entity?

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
            // Entity changed its chunk position without changing instances. Remove
            // it from old cell and insert it in new cell.

            // TODO: skip marker entity?

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
            // we need to do.
        }
    }

    let mut scratch_1 = vec![];
    let mut scratch_2 = vec![];

    for instance in &mut instances {
        let instance = instance.into_inner();

        for (&pos, cell) in &mut instance.partition {
            // Cache chunk update packets into the packet buffer of this cell.
            if let Some(chunk) = &mut cell.chunk {
                let writer = PacketWriter::new(
                    &mut cell.packet_buf,
                    server.compression_threshold(),
                    &mut scratch_2,
                );

                chunk.write_update_packets(writer, &mut scratch_1, pos, &instance.info);
            }

            // Cache entity update packets into the packet buffer of this cell.
            for &id in &cell.entities {
                let (_, mut entity, despawned) = entities
                    .get_mut(id)
                    .expect("missing entity in partition cell");

                if despawned.is_some() {
                    continue;
                }

                let start = cell.packet_buf.len();

                let writer = PacketWriter::new(
                    &mut cell.packet_buf,
                    server.compression_threshold(),
                    &mut scratch_2,
                );

                entity.write_update_packets(writer, &mut scratch_1);

                let end = cell.packet_buf.len();

                entity.self_update_range = start..end;
            }
        }
    }
}

pub(crate) fn update_instances_post_client(mut instances: Query<&mut Instance>) {
    for mut instance in &mut instances {
        instance.partition.retain(|_, cell| {
            cell.packet_buf.clear();
            cell.chunk_removed = false;
            cell.incoming.clear();
            cell.outgoing.clear();

            if let Some(chunk) = &mut cell.chunk {
                chunk.update_post_client();
            }

            cell.chunk.is_some() || cell.entities.len() > 0
        });

        instance.packet_buf.clear();
    }
}

pub(crate) fn check_instance_invariants(instances: Query<&Instance>, entities: Query<&McEntity>) {
    #[cfg(debug_assertions)]
    for instance in &instances {
        for (pos, cell) in &instance.partition {
            for &id in &cell.entities {
                assert!(
                    entities.get(id).is_ok(),
                    "instance contains an entity that does not exist at {pos:?}"
                );
            }
        }
    }

    let _ = instances;
    let _ = entities;
}
