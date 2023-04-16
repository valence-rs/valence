use std::borrow::Cow;
use std::collections::hash_map::Entry;
use std::collections::BTreeSet;
use std::iter::FusedIterator;
use std::mem;

use bevy_app::{CoreSet, Plugin};
use bevy_ecs::prelude::*;
use bevy_ecs::query::WorldQuery;
pub use chunk::{Block, BlockEntity, BlockMut, BlockRef, Chunk};
pub use chunk_entry::*;
use glam::{DVec3, Vec3};
use num_integer::div_ceil;
use rustc_hash::FxHashMap;
use valence_protocol::array::LengthPrefixedArray;
use valence_protocol::block_pos::BlockPos;
use valence_protocol::byte_angle::ByteAngle;
use valence_protocol::ident::Ident;
use valence_protocol::packet::s2c::play::particle::Particle;
use valence_protocol::packet::s2c::play::{
    EntityAnimationS2c, EntityPositionS2c, EntitySetHeadYawS2c, EntityStatusS2c,
    EntityTrackerUpdateS2c, EntityVelocityUpdateS2c, MoveRelative, OverlayMessageS2c, ParticleS2c,
    PlaySoundS2c, Rotate, RotateAndMoveRelative,
};
use valence_protocol::sound::Sound;
use valence_protocol::text::Text;
use valence_protocol::types::SoundCategory;
use valence_protocol::var_int::VarInt;
use valence_protocol::Packet;

use crate::biome::Biome;
use crate::client::FlushPacketsSet;
use crate::component::{Despawned, Location, Look, OldLocation, OldPosition, OnGround, Position};
use crate::dimension::DimensionType;
use crate::entity::{
    EntityAnimations, EntityId, EntityKind, EntityStatuses, HeadYaw, InitEntitiesSet,
    PacketByteRange, TrackedData, Velocity,
};
use crate::packet::{PacketWriter, WritePacket};
use crate::util::velocity_to_packet_units;
use crate::view::ChunkPos;
use crate::Server;

mod chunk;
mod chunk_entry;
mod paletted_container;

/// An Instance represents a Minecraft world, which consist of [`Chunk`]s.
/// It manages updating clients when chunks change, and caches chunk and entity
/// update packets on a per-chunk basis.
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
    dimension_type_name: Ident<String>,
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
    pub fn new(
        dimension_type_name: impl Into<Ident<String>>,
        dimensions: &Query<&DimensionType>,
        biomes: &Query<&Biome>,
        server: &Server,
    ) -> Self {
        let dimension_type_name = dimension_type_name.into();

        let Some(dim) = dimensions.iter().find(|d| d.name == dimension_type_name) else {
            panic!("missing dimension type with name \"{dimension_type_name}\"")
        };

        assert!(dim.height > 0, "invalid dimension height of {}", dim.height);

        let light_section_count = (dim.height / 16 + 2) as usize;

        let mut sky_light_mask = vec![0; div_ceil(light_section_count, 16)];

        for i in 0..light_section_count {
            sky_light_mask[i / 64] |= 1 << (i % 64);
        }

        Self {
            partition: FxHashMap::default(),
            info: InstanceInfo {
                dimension_type_name,
                section_count: (dim.height / 16) as usize,
                min_y: dim.min_y,
                biome_registry_len: biomes.iter().count(),
                compression_threshold: server.compression_threshold(),
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

    /// TODO: Temporary hack for unit testing. Do not use!
    #[doc(hidden)]
    pub fn new_unit_testing(
        dimension_type_name: impl Into<Ident<String>>,
        server: &Server,
    ) -> Self {
        Self {
            partition: FxHashMap::default(),
            info: InstanceInfo {
                dimension_type_name: dimension_type_name.into(),
                section_count: 24,
                min_y: -64,
                biome_registry_len: 1,
                compression_threshold: server.compression_threshold(),
                filler_sky_light_mask: vec![].into(),
                filler_sky_light_arrays: vec![].into(),
            },
            packet_buf: vec![],
            scratch: vec![],
        }
    }

    pub fn dimension_type_name(&self) -> Ident<&str> {
        self.info.dimension_type_name.as_str_ident()
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

    /// Gets a reference to the block at an absolute block position in world
    /// space. Only works for blocks in loaded chunks.
    ///
    /// If the position is not inside of a chunk, then [`Option::None`] is
    /// returned.
    pub fn block(&self, pos: impl Into<BlockPos>) -> Option<BlockRef> {
        let pos = pos.into();

        let Some(y) = pos.y.checked_sub(self.info.min_y).and_then(|y| y.try_into().ok()) else {
            return None;
        };

        if y >= self.info.section_count * 16 {
            return None;
        }

        let Some(chunk) = self.chunk(ChunkPos::from_block_pos(pos)) else {
            return None;
        };

        Some(chunk.block(
            pos.x.rem_euclid(16) as usize,
            y,
            pos.z.rem_euclid(16) as usize,
        ))
    }

    /// Gets a mutable reference to the block at an absolute block position in
    /// world space. Only works for blocks in loaded chunks.
    ///
    /// If the position is not inside of a chunk, then [`Option::None`] is
    /// returned.
    pub fn block_mut(&mut self, pos: impl Into<BlockPos>) -> Option<BlockMut> {
        let pos = pos.into();

        let Some(y) = pos.y.checked_sub(self.info.min_y).and_then(|y| y.try_into().ok()) else {
            return None;
        };

        if y >= self.info.section_count * 16 {
            return None;
        }

        let Some(chunk) = self.chunk_mut(ChunkPos::from_block_pos(pos)) else {
            return None;
        };

        Some(chunk.block_mut(
            pos.x.rem_euclid(16) as usize,
            y,
            pos.z.rem_euclid(16) as usize,
        ))
    }

    /// Sets the block at an absolute block position in world space. The
    /// previous block at the position is returned.
    ///
    /// If the position is not within a loaded chunk or otherwise out of bounds,
    /// then [`Option::None`] is returned with no effect.
    pub fn set_block(
        &mut self,
        pos: impl Into<BlockPos>,
        block: impl Into<Block>,
    ) -> Option<Block> {
        let pos = pos.into();

        let Some(y) = pos.y.checked_sub(self.info.min_y).and_then(|y| y.try_into().ok()) else {
            return None;
        };

        if y >= self.info.section_count * 16 {
            return None;
        }

        let Some(chunk) = self.chunk_mut(ChunkPos::from_block_pos(pos)) else {
            return None;
        };

        Some(chunk.set_block(
            pos.x.rem_euclid(16) as usize,
            y,
            pos.z.rem_euclid(16) as usize,
            block,
        ))
    }

    /// Writes a packet into the global packet buffer of this instance. All
    /// clients in the instance will receive the packet.
    ///
    /// This is more efficient than sending the packet to each client
    /// individually.
    pub fn write_packet<'a, P>(&mut self, pkt: &P)
    where
        P: Packet<'a>,
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
    pub fn write_packet_at<'a, P>(&mut self, pkt: &P, pos: impl Into<ChunkPos>)
    where
        P: Packet<'a>,
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
                particle: Cow::Borrowed(particle),
                long_distance,
                position: position.into(),
                offset: offset.into().into(),
                max_speed,
                count,
            },
            ChunkPos::from_dvec3(position),
        );
    }

    /// Plays a sound effect at the given position in the world. The sound
    /// effect is audible to all players in the instance with the
    /// appropriate chunk in view.
    pub fn play_sound(
        &mut self,
        sound: Sound,
        category: SoundCategory,
        position: impl Into<DVec3>,
        volume: f32,
        pitch: f32,
    ) {
        let position = position.into();

        self.write_packet_at(
            &PlaySoundS2c {
                id: sound.to_id(),
                category,
                position: (position * 8.0).as_ivec3().into(),
                volume,
                pitch,
                seed: rand::random(),
            },
            ChunkPos::from_dvec3(position),
        );
    }

    /// Sets the action bar text of all players in the instance.
    pub fn set_action_bar(&mut self, text: impl Into<Text>) {
        self.write_packet(&OverlayMessageS2c {
            action_bar_text: text.into().into(),
        });
    }
}

pub(crate) struct InstancePlugin;

#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct WriteUpdatePacketsToInstancesSet;

impl Plugin for InstancePlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.configure_set(
            WriteUpdatePacketsToInstancesSet
                .after(InitEntitiesSet)
                .in_base_set(CoreSet::PostUpdate),
        )
        .add_system(
            update_entity_cell_positions
                .before(WriteUpdatePacketsToInstancesSet)
                .in_base_set(CoreSet::PostUpdate),
        )
        .add_system(write_update_packets_to_instances.in_set(WriteUpdatePacketsToInstancesSet))
        .add_system(
            update_instances_post_client
                .after(FlushPacketsSet)
                .in_base_set(CoreSet::PostUpdate),
        );

        #[cfg(debug_assertions)]
        app.add_system(check_instance_invariants.in_base_set(CoreSet::PostUpdate));
    }
}

/// Handles entities moving from one chunk to another.
fn update_entity_cell_positions(
    entities: Query<
        (
            Entity,
            &Position,
            &OldPosition,
            &Location,
            &OldLocation,
            Option<&Despawned>,
        ),
        (With<EntityKind>, Or<(Changed<Position>, With<Despawned>)>),
    >,
    mut instances: Query<&mut Instance>,
) {
    for (entity, pos, old_pos, loc, old_loc, despawned) in &entities {
        let pos = ChunkPos::at(pos.0.x, pos.0.z);
        let old_pos = ChunkPos::at(old_pos.get().x, old_pos.get().z);

        if despawned.is_some() {
            // Entity was deleted. Remove it from the chunk it was in, if it was in a chunk
            // at all.
            if let Ok(mut old_instance) = instances.get_mut(old_loc.get()) {
                if let Some(old_cell) = old_instance.partition.get_mut(&old_pos) {
                    if old_cell.entities.remove(&entity) {
                        old_cell.outgoing.push((entity, None));
                    }
                }
            }
        } else if old_loc.get() != loc.0 {
            // Entity changed the instance it is in. Remove it from old cell and
            // insert it in the new cell.

            // TODO: skip marker entity?

            if let Ok(mut old_instance) = instances.get_mut(old_loc.get()) {
                if let Some(old_cell) = old_instance.partition.get_mut(&old_pos) {
                    if old_cell.entities.remove(&entity) {
                        old_cell.outgoing.push((entity, None));
                    }
                }
            }

            if let Ok(mut instance) = instances.get_mut(loc.0) {
                match instance.partition.entry(pos) {
                    Entry::Occupied(oe) => {
                        let cell = oe.into_mut();
                        if cell.entities.insert(entity) {
                            cell.incoming.push((entity, None));
                        }
                    }
                    Entry::Vacant(ve) => {
                        ve.insert(PartitionCell {
                            chunk: None,
                            chunk_removed: false,
                            entities: BTreeSet::from([entity]),
                            incoming: vec![(entity, None)],
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

            if let Ok(mut instance) = instances.get_mut(loc.0) {
                if let Some(old_cell) = instance.partition.get_mut(&old_pos) {
                    if old_cell.entities.remove(&entity) {
                        old_cell.outgoing.push((entity, Some(pos)));
                    }
                }

                match instance.partition.entry(pos) {
                    Entry::Occupied(oe) => {
                        let cell = oe.into_mut();
                        if cell.entities.insert(entity) {
                            cell.incoming.push((entity, Some(old_pos)));
                        }
                    }
                    Entry::Vacant(ve) => {
                        ve.insert(PartitionCell {
                            chunk: None,
                            chunk_removed: false,
                            entities: BTreeSet::from([entity]),
                            incoming: vec![(entity, Some(old_pos))],
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
}

/// Writes update packets from entities and chunks into each cell's packet
/// buffer.
fn write_update_packets_to_instances(
    mut instances: Query<&mut Instance>,
    mut entities: Query<UpdateEntityQuery, (With<EntityKind>, Without<Despawned>)>,
    server: Res<Server>,
) {
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
            for &entity in &cell.entities {
                let mut entity = entities
                    .get_mut(entity)
                    .expect("missing entity in partition cell");

                let start = cell.packet_buf.len();

                let writer = PacketWriter::new(
                    &mut cell.packet_buf,
                    server.compression_threshold(),
                    &mut scratch_2,
                );

                entity.write_update_packets(writer);

                let end = cell.packet_buf.len();

                entity.packet_byte_range.0 = start..end;
            }
        }
    }
}

#[derive(WorldQuery)]
#[world_query(mutable)]
struct UpdateEntityQuery {
    id: &'static EntityId,
    pos: &'static Position,
    old_pos: &'static OldPosition,
    loc: &'static Location,
    old_loc: &'static OldLocation,
    look: Ref<'static, Look>,
    head_yaw: Ref<'static, HeadYaw>,
    on_ground: &'static OnGround,
    velocity: Ref<'static, Velocity>,
    tracked_data: &'static TrackedData,
    statuses: &'static EntityStatuses,
    animations: &'static EntityAnimations,
    packet_byte_range: &'static mut PacketByteRange,
}

impl UpdateEntityQueryItem<'_> {
    fn write_update_packets(&self, mut writer: impl WritePacket) {
        // TODO: @RJ I saw you're using UpdateEntityPosition and UpdateEntityRotation sometimes. These two packets are actually broken on the client and will erase previous position/rotation https://bugs.mojang.com/browse/MC-255263 -Moulberry

        let entity_id = VarInt(self.id.get());

        let position_delta = self.pos.0 - self.old_pos.get();
        let needs_teleport = position_delta.abs().max_element() >= 8.0;
        let changed_position = self.pos.0 != self.old_pos.get();

        if changed_position && !needs_teleport && self.look.is_changed() {
            writer.write_packet(&RotateAndMoveRelative {
                entity_id,
                delta: (position_delta * 4096.0).to_array().map(|v| v as i16),
                yaw: ByteAngle::from_degrees(self.look.yaw),
                pitch: ByteAngle::from_degrees(self.look.pitch),
                on_ground: self.on_ground.0,
            });
        } else {
            if changed_position && !needs_teleport {
                writer.write_packet(&MoveRelative {
                    entity_id,
                    delta: (position_delta * 4096.0).to_array().map(|v| v as i16),
                    on_ground: self.on_ground.0,
                });
            }

            if self.look.is_changed() {
                writer.write_packet(&Rotate {
                    entity_id,
                    yaw: ByteAngle::from_degrees(self.look.yaw),
                    pitch: ByteAngle::from_degrees(self.look.pitch),
                    on_ground: self.on_ground.0,
                });
            }
        }

        if needs_teleport {
            writer.write_packet(&EntityPositionS2c {
                entity_id,
                position: self.pos.0.to_array(),
                yaw: ByteAngle::from_degrees(self.look.yaw),
                pitch: ByteAngle::from_degrees(self.look.pitch),
                on_ground: self.on_ground.0,
            });
        }

        if self.velocity.is_changed() {
            writer.write_packet(&EntityVelocityUpdateS2c {
                entity_id,
                velocity: self.velocity.to_packet_units(),
            });
        }

        if self.head_yaw.is_changed() {
            writer.write_packet(&EntitySetHeadYawS2c {
                entity_id,
                head_yaw: ByteAngle::from_degrees(self.head_yaw.0),
            });
        }

        if let Some(update_data) = self.tracked_data.update_data() {
            writer.write_packet(&EntityTrackerUpdateS2c {
                entity_id,
                metadata: update_data.into(),
            });
        }

        if self.statuses.0 != 0 {
            for i in 0..mem::size_of_val(self.statuses) {
                if (self.statuses.0 >> i) & 1 == 1 {
                    writer.write_packet(&EntityStatusS2c {
                        entity_id: entity_id.0,
                        entity_status: i as u8,
                    });
                }
            }
        }

        if self.animations.0 != 0 {
            for i in 0..mem::size_of_val(self.animations) {
                if (self.animations.0 >> i) & 1 == 1 {
                    writer.write_packet(&EntityAnimationS2c {
                        entity_id,
                        animation: i as u8,
                    });
                }
            }
        }
    }
}

fn update_instances_post_client(mut instances: Query<&mut Instance>) {
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

#[cfg(debug_assertions)]
fn check_instance_invariants(instances: Query<&Instance>, entities: Query<(), With<EntityKind>>) {
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
}
