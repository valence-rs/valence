use std::collections::hash_map::Entry;
use std::collections::BTreeSet;
use std::iter::FusedIterator;

use bevy_ecs::prelude::*;
pub use chunk_entry::*;
use glam::{DVec3, Vec3};
use num::integer::div_ceil;
use rustc_hash::FxHashMap;
use valence_protocol::block::BlockState;
use valence_protocol::packets::s2c::particle::{Particle, ParticleS2c};
use valence_protocol::packets::s2c::play::SetActionBarText;
use valence_protocol::{BlockPos, EncodePacket, LengthPrefixedArray, Text};

use crate::dimension::DimensionId;
use crate::entity::McEntity;
pub use crate::instance::chunk::Chunk;
use crate::packet::{PacketWriter, WritePacket};
use crate::server::{Server, SharedServer};
use crate::view::ChunkPos;
use crate::Despawned;

mod chunk;
mod chunk_entry;
mod paletted_container;

/// An Instance represents a Minecraft world, which consist of [`Chunk`]s.
/// It manages updating clients when chunks change, and caches chunk and entity
/// update packets on a per-chunk basis.
///
/// To create a new instance, use [`SharedServer::new_instance`].
/// ```
/// use bevy_app::prelude::*;
/// use valence::prelude::*;
///
/// let mut app = App::new();
/// app.add_plugin(ServerPlugin::new(()));
/// let server = app.world.get_resource::<Server>().unwrap();
/// let instance = server.new_instance(DimensionId::default());
/// ```
/// Now you can actually spawn a new [`Entity`] with `instance`.
/// ```
/// # use bevy_app::prelude::*;
/// # use valence::prelude::*;
/// # let mut app = App::new();
/// # app.add_plugin(ServerPlugin::new(()));
/// # let server = app.world.get_resource::<Server>().unwrap();
/// # let instance = server.new_instance(DimensionId::default());
/// let instance_entity = app.world.spawn(instance);
/// ```
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

    pub fn dimension(&self) -> DimensionId {
        self.info.dimension
    }

    pub fn section_count(&self) -> usize {
        self.info.section_count
    }

    /// Get a reference to the chunk at the given position, if it is loaded.
    pub fn chunk(&self, pos: impl Into<ChunkPos>) -> Option<&Chunk<true>> {
        self.partition
            .get(&pos.into())
            .and_then(|p| p.chunk.as_ref())
    }

    /// Get a mutable reference to the chunk at the given position, if it is
    /// loaded.
    pub fn chunk_mut(&mut self, pos: impl Into<ChunkPos>) -> Option<&mut Chunk<true>> {
        self.partition
            .get_mut(&pos.into())
            .and_then(|p| p.chunk.as_mut())
    }

    /// Insert a chunk into the instance at the given position. This effectively
    /// loads the Chunk.
    pub fn insert_chunk(&mut self, pos: impl Into<ChunkPos>, chunk: Chunk) -> Option<Chunk> {
        match self.chunk_entry(pos) {
            ChunkEntry::Occupied(mut oe) => Some(oe.insert(chunk)),
            ChunkEntry::Vacant(ve) => {
                ve.insert(chunk);
                None
            }
        }
    }

    /// Unload the chunk at the given position, if it is loaded. Returns the
    /// chunk if it was loaded.
    pub fn remove_chunk(&mut self, pos: impl Into<ChunkPos>) -> Option<Chunk> {
        match self.chunk_entry(pos) {
            ChunkEntry::Occupied(oe) => Some(oe.remove()),
            ChunkEntry::Vacant(_) => None,
        }
    }

    /// Unload all chunks in this instance.
    pub fn clear_chunks(&mut self) {
        self.retain_chunks(|_, _| false)
    }

    /// Retain only the chunks for which the given predicate returns `true`.
    pub fn retain_chunks<F>(&mut self, mut f: F)
    where
        F: FnMut(ChunkPos, &mut Chunk<true>) -> bool,
    {
        for (&pos, cell) in &mut self.partition {
            if let Some(chunk) = &mut cell.chunk {
                if !f(pos, chunk) {
                    cell.chunk = None;
                    cell.chunk_removed = true;
                }
            }
        }
    }

    /// Get a [`ChunkEntry`] for the given position.
    pub fn chunk_entry(&mut self, pos: impl Into<ChunkPos>) -> ChunkEntry {
        ChunkEntry::new(self.info.section_count, self.partition.entry(pos.into()))
    }

    /// Get an iterator over all loaded chunks in the instance. The order of the
    /// chunks is undefined.
    pub fn chunks(&self) -> impl FusedIterator<Item = (ChunkPos, &Chunk<true>)> + Clone + '_ {
        self.partition
            .iter()
            .flat_map(|(&pos, par)| par.chunk.as_ref().map(|c| (pos, c)))
    }

    /// Get an iterator over all loaded chunks in the instance, mutably. The
    /// order of the chunks is undefined.
    pub fn chunks_mut(&mut self) -> impl FusedIterator<Item = (ChunkPos, &mut Chunk<true>)> + '_ {
        self.partition
            .iter_mut()
            .flat_map(|(&pos, par)| par.chunk.as_mut().map(|c| (pos, c)))
    }

    /// Optimizes the memory usage of the instance.
    pub fn optimize(&mut self) {
        for (_, chunk) in self.chunks_mut() {
            chunk.optimize();
        }

        self.partition.shrink_to_fit();
        self.packet_buf.shrink_to_fit();
    }

    /// Gets the block state at an absolute block position in world space. Only
    /// works for blocks in loaded chunks.
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

    /// Writes arbitrary packet data into the global packet buffer of this
    /// instance. All clients in the instance will receive the packet data.
    ///
    /// The packet data must be properly compressed for the current compression
    /// threshold but never encrypted. Don't use this function unless you know
    /// what you're doing. Consider using [`Self::write_packet`] instead.
    pub fn write_packet_bytes(&mut self, bytes: &[u8]) {
        self.packet_buf.extend_from_slice(bytes)
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

    /// Writes arbitrary packet data to all clients in view of `pos` in this
    /// instance. Has no effect if there is no chunk at `pos`.
    ///
    /// The packet data must be properly compressed for the current compression
    /// threshold but never encrypted. Don't use this function unless you know
    /// what you're doing. Consider using [`Self::write_packet`] instead.
    pub fn write_packet_bytes_at(&mut self, bytes: &[u8], pos: impl Into<ChunkPos>) {
        let pos = pos.into();
        if let Some(cell) = self.partition.get_mut(&pos) {
            if cell.chunk.is_some() {
                cell.packet_buf.extend_from_slice(bytes);
            }
        }
    }

    /// Puts a particle effect at the given position in the world. The particle
    /// effect is visible to all players in the instance with the
    /// appropriate chunk in view.
    pub fn play_particle(
        &mut self,
        particle: &Particle,
        long_distance: bool,
        position: impl Into<DVec3>,
        offset: impl Into<Vec3>,
        max_speed: f32,
        count: i32,
    ) {
        let position = position.into();

        self.write_packet_at(
            &ParticleS2c {
                particle: particle.clone(),
                long_distance,
                position: position.into(),
                offset: offset.into().into(),
                max_speed,
                count,
            },
            ChunkPos::from_dvec3(position),
        );
    }

    /// Sets the action bar text of all players in the instance.
    pub fn set_action_bar(&mut self, text: impl Into<Text>) {
        self.write_packet(&SetActionBarText {
            action_bar_text: text.into().into(),
        });
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

                chunk.clear_viewed();
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

            cell.chunk.is_some() || !cell.entities.is_empty()
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
