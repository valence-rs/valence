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

use std::mem;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::query::{Has, WorldQuery};
use chunk::loaded::ChunkState;
use valence_core::chunk_pos::ChunkPos;
use valence_core::despawn::Despawned;
use valence_core::protocol::byte_angle::ByteAngle;
use valence_core::protocol::encode::WritePacket;
use valence_core::protocol::var_int::VarInt;
use valence_entity::packet::{
    EntityAnimationS2c, EntityPositionS2c, EntitySetHeadYawS2c, EntityStatusS2c,
    EntityTrackerUpdateS2c, EntityVelocityUpdateS2c, MoveRelativeS2c, RotateAndMoveRelativeS2c,
    RotateS2c,
};
use valence_entity::{
    EntityAnimations, EntityId, EntityKind, EntityStatuses, HeadYaw, InLoadedChunk,
    InitEntitiesSet, Location, Look, OldLocation, OldPosition, OnGround, PacketByteRange, Position,
    TrackedData, UpdateTrackedDataSet, Velocity,
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
            update_entity_chunk_positions.before(WriteUpdatePacketsToInstancesSet),
        )
        .add_systems(
            PostUpdate,
            write_update_packets_to_chunks
                .after(update_entity_chunk_positions)
                .in_set(WriteUpdatePacketsToInstancesSet),
        )
        .add_systems(
            PostUpdate,
            clear_instance_changes.in_set(ClearInstanceChangesSet),
        );

        // #[cfg(debug_assertions)]
        // app.add_systems(PostUpdate, check_instance_invariants);
    }
}

/// Handles entities moving from one chunk to another.
fn update_entity_chunk_positions(
    mut entities: Query<
        (
            Entity,
            &Position,
            &OldPosition,
            &Location,
            &OldLocation,
            &mut InLoadedChunk,
            Has<Despawned>,
        ),
        (
            With<EntityKind>,
            Or<(Changed<Position>, Changed<Location>, With<Despawned>)>,
        ),
    >,
    mut instances: Query<&mut Instance>,
) {
    for (entity, pos, old_pos, loc, old_loc, mut in_loaded_chunk, despawned) in &mut entities {
        let pos = ChunkPos::at(pos.0.x, pos.0.z);
        let old_pos = ChunkPos::at(old_pos.get().x, old_pos.get().z);

        if despawned {
            // Entity was deleted. Remove it from the chunk it was in.
            if in_loaded_chunk.get() {
                if let Ok(mut old_inst) = instances.get_mut(old_loc.get()) {
                    if let Some(old_chunk) = old_inst.chunk_mut(old_pos) {
                        if old_chunk.entities.remove(&entity) {
                            old_chunk.outgoing_entities.push((entity, None));
                        }
                    }
                }

                in_loaded_chunk.set(false);
            }
        } else if old_loc.get() != loc.0 {
            // Entity changed the instance it is in. Remove it from old chunk and
            // insert it in the new chunk.

            if let Ok(mut old_inst) = instances.get_mut(old_loc.get()) {
                if in_loaded_chunk.get() {
                    if let Some(old_chunk) = old_inst.chunk_mut(old_pos) {
                        if old_chunk.entities.remove(&entity) {
                            old_chunk.outgoing_entities.push((entity, None));
                        }
                    }
                }
            }

            in_loaded_chunk.set(false);

            if let Ok(mut inst) = instances.get_mut(loc.0) {
                if let Some(chunk) = inst.chunk_mut(pos) {
                    if chunk.entities.insert(entity) {
                        chunk.incoming_entities.push((entity, None));
                    }
                    in_loaded_chunk.set(true);
                }
            }
        } else if pos != old_pos {
            // Entity changed its chunk position without changing instances. Insert it in
            // the new chunk and remove it from the old chunk.

            if let Ok(mut inst) = instances.get_mut(loc.0) {
                if let Some(chunk) = inst.chunk_mut(pos) {
                    if chunk.entities.insert(entity) {
                        let from = if in_loaded_chunk.get() {
                            Some(old_pos)
                        } else {
                            None
                        };

                        chunk.incoming_entities.push((entity, from));
                    }

                    in_loaded_chunk.set(true);
                } else {
                    in_loaded_chunk.set(false);
                }

                if let Some(old_chunk) = inst.chunk_mut(old_pos) {
                    if old_chunk.entities.remove(&entity) {
                        let to = if in_loaded_chunk.get() {
                            Some(pos)
                        } else {
                            None
                        };

                        old_chunk.outgoing_entities.push((entity, to));
                    }
                }
            }
        } else {
            // The entity didn't change its chunk position, so there's nothing
            // we need to do.
        }
    }
}

/// Writes update packets from entities and chunks into each chunk's packet
/// buffer.
fn write_update_packets_to_chunks(
    mut instances: Query<&mut Instance>,
    mut entities: Query<UpdateEntityQuery, (With<EntityKind>, Without<Despawned>)>,
) {
    for inst in &mut instances {
        let inst = inst.into_inner();

        for (&pos, chunk) in &mut inst.chunks {
            chunk.update_pre_client(pos, &inst.info, &mut entities)
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
    packet_byte_range: Option<&'static mut PacketByteRange>,
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
    for mut inst in &mut instances {
        inst.retain_chunks(|_, chunk| {
            chunk.update_post_client();
            chunk.state() != ChunkState::Removed
        });

        inst.packet_buf.clear();
    }
}

// #[cfg(debug_assertions)]
// fn check_instance_invariants(instances: Query<&Instance>, entities: Query<(),
// With<EntityKind>>) {     for instance in &instances {
//         for (pos, cell) in &instance.partition {
//             for &id in &cell.entities {
//                 assert!(
//                     entities.get(id).is_ok(),
//                     "instance contains an entity that does not exist at
// {pos:?}"                 );
//             }
//         }
//     }
// }
