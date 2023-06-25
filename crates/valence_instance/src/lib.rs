#![doc = include_str!("../README.md")]
#![deny(
    rustdoc::broken_intra_doc_links,
    rustdoc::private_intra_doc_links,
    rustdoc::missing_crate_level_docs,
    rustdoc::invalid_codeblock_attributes,
    rustdoc::invalid_rust_codeblocks,
    rustdoc::bare_urls,
    rustdoc::invalid_html_tags
)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unused_lifetimes,
    unused_import_braces,
    unreachable_pub,
    clippy::dbg_macro
)]
#![allow(clippy::type_complexity)]

use std::borrow::Cow;
use std::collections::hash_map::Entry;
use std::collections::BTreeSet;
use std::iter::FusedIterator;
use std::mem;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::query::{Has, WorldQuery};
use chunk::LoadedChunk;
use glam::{DVec3, Vec3};
use num_integer::div_ceil;
use rustc_hash::FxHashMap;
use valence_biome::BiomeRegistry;
use valence_core::block_pos::BlockPos;
use valence_core::chunk_pos::ChunkPos;
use valence_core::despawn::Despawned;
use valence_core::ident::Ident;
use valence_core::particle::{Particle, ParticleS2c};
use valence_core::protocol::array::LengthPrefixedArray;
use valence_core::protocol::byte_angle::ByteAngle;
use valence_core::protocol::encode::{PacketWriter, WritePacket};
use valence_core::protocol::packet::sound::{PlaySoundS2c, Sound, SoundCategory};
use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::{Encode, Packet};
use valence_core::Server;
use valence_dimension::DimensionTypeRegistry;
use valence_entity::packet::{
    EntityAnimationS2c, EntityPositionS2c, EntitySetHeadYawS2c, EntityStatusS2c,
    EntityTrackerUpdateS2c, EntityVelocityUpdateS2c, MoveRelativeS2c, RotateAndMoveRelativeS2c,
    RotateS2c,
};
use valence_entity::{
    EntityAnimations, EntityId, EntityKind, EntityStatuses, HeadYaw, InitEntitiesSet, Location,
    Look, OldLocation, OldPosition, OnGround, PacketByteRange, Position, TrackedData,
    UpdateTrackedDataSet, Velocity,
};

pub mod chunk;
mod instance;
pub mod packet;

pub use instance::*;

pub struct InstancePlugin;

/// When Minecraft entity changes are written to the packet buffers of chunks.
/// Systems that modify entites should run _before_ this. Systems that read from
/// the packet buffer of chunks should run _after_ this.
///
/// This set lives in [`PostUpdate`].
#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct WriteUpdatePacketsToInstancesSet;

/// When instances are updated and changes from the current tick are cleared.
/// Systems that read changes from instances should run _before_ this.
///
/// This set lives in [`PostUpdate`].
#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct ClearInstanceChangesSet;

/*
impl Plugin for InstancePlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            PostUpdate,
            (
                WriteUpdatePacketsToInstancesSet
                    .after(InitEntitiesSet)
                    .after(UpdateTrackedDataSet),
                ClearInstanceChangesSet.after(WriteUpdatePacketsToInstancesSet),
            ),
        )
        .add_systems(
            PostUpdate,
            // This can run at the same time as entity init because we're only looking at position
            // + location.
            update_entity_cell_positions.before(WriteUpdatePacketsToInstancesSet),
        )
        .add_systems(
            PostUpdate,
            write_update_packets_to_instances
                .after(update_entity_cell_positions)
                .in_set(WriteUpdatePacketsToInstancesSet),
        )
        .add_systems(
            PostUpdate,
            clear_instance_changes.in_set(ClearInstanceChangesSet),
        );

        #[cfg(debug_assertions)]
        app.add_systems(PostUpdate, check_instance_invariants);
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
            Has<Despawned>,
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
    for instance in &mut instances {
        let instance = instance.into_inner();

        for (&pos, cell) in &mut instance.partition {
            // Cache chunk update packets into the packet buffer of this cell.
            if let Some(chunk) = &mut cell.chunk {
                let writer =
                    PacketWriter::new(&mut cell.packet_buf, server.compression_threshold());

                chunk.write_update_packets(writer, pos, &instance.info);

                chunk.clear_viewed();
            }

            // Cache entity update packets into the packet buffer of this cell.
            for &entity in &cell.entities {
                let mut entity = entities
                    .get_mut(entity)
                    .expect("missing entity in partition cell");

                let start = cell.packet_buf.len();

                let writer =
                    PacketWriter::new(&mut cell.packet_buf, server.compression_threshold());

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
            writer.write_packet(&RotateAndMoveRelativeS2c {
                entity_id,
                delta: (position_delta * 4096.0).to_array().map(|v| v as i16),
                yaw: ByteAngle::from_degrees(self.look.yaw),
                pitch: ByteAngle::from_degrees(self.look.pitch),
                on_ground: self.on_ground.0,
            });
        } else {
            if changed_position && !needs_teleport {
                writer.write_packet(&MoveRelativeS2c {
                    entity_id,
                    delta: (position_delta * 4096.0).to_array().map(|v| v as i16),
                    on_ground: self.on_ground.0,
                });
            }

            if self.look.is_changed() {
                writer.write_packet(&RotateS2c {
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
                position: self.pos.0,
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

fn clear_instance_changes(mut instances: Query<&mut Instance>) {
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

*/
