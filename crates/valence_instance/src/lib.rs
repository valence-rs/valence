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

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::query::Has;
use chunk::loaded::ChunkState;
use message::MessageCondition;
use valence_core::chunk_pos::ChunkPos;
use valence_core::despawn::Despawned;
use valence_entity::packet::EntitiesDestroyS2c;
use valence_entity::query::{EntityInitQuery, UpdateEntityQuery};
use valence_entity::{
    EntityId, EntityKind, InitEntitiesSet, Location, OldLocation, OldPosition, Position,
    UpdateTrackedDataSet,
};

pub mod chunk;
mod instance;
pub mod message;
pub mod packet;

pub use chunk::{Block, BlockRef};
pub use instance::*;

pub struct InstancePlugin;

/// When Minecraft entity changes are written to the packet buffers of
/// instances. Systems that modify entites should run _before_ this. Systems
/// that read from the packet buffer of instances should run _after_ this.
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
            (add_orphaned_entities, update_entity_chunk_positions)
                .chain()
                .before(WriteUpdatePacketsToInstancesSet),
        )
        .add_systems(
            PostUpdate,
            write_update_packets_per_chunk
                .after(update_entity_chunk_positions)
                .in_set(WriteUpdatePacketsToInstancesSet),
        )
        .add_systems(
            PostUpdate,
            update_post_client.in_set(ClearInstanceChangesSet),
        );
    }
}

/// Marker component for entities that are not contained in a chunk.
#[derive(Component, Debug)]
struct Orphaned;

/// Attempts to add orphaned entities to the chunk they're positioned in.
fn add_orphaned_entities(
    entities: Query<(Entity, &Position, &OldPosition, &Location, EntityInitQuery), With<Orphaned>>,
    mut instances: Query<&mut Instance>,
    mut commands: Commands,
) {
    for (entity, pos, old_pos, loc, init_item) in &entities {
        if let Ok(mut inst) = instances.get_mut(loc.0) {
            let pos = ChunkPos::at(pos.0.x, pos.0.z);

            if let Some(chunk) = inst.chunk_mut(pos) {
                if chunk.entities.insert(entity) && chunk.is_viewed_mut() {
                    inst.message_buf.send(MessageCondition::View { pos }, |w| {
                        init_item.write_init_packets(old_pos.get(), w)
                    });
                }

                // Entity is no longer orphaned.
                commands.entity(entity).remove::<Orphaned>();
            }
        }
    }
}

/// Handles entities moving from one chunk to another.
fn update_entity_chunk_positions(
    entities: Query<
        (
            Entity,
            &Position,
            &OldPosition,
            &Location,
            &OldLocation,
            EntityInitQuery,
            Has<Despawned>,
        ),
        (Or<(Changed<Position>, Changed<Location>, With<Despawned>)>,),
    >,
    entity_ids: Query<&EntityId>,
    mut instances: Query<&mut Instance>,
    mut commands: Commands,
) {
    for (entity, pos, old_pos, loc, old_loc, init_item, despawned) in &entities {
        let chunk_pos = ChunkPos::at(pos.0.x, pos.0.z);
        let old_chunk_pos = ChunkPos::at(old_pos.get().x, old_pos.get().z);

        if despawned {
            // Entity was deleted. Remove it from the chunk it was in.
            if let Ok(mut old_inst) = instances.get_mut(old_loc.get()) {
                if let Some(old_chunk) = old_inst.chunk_mut(old_chunk_pos) {
                    if old_chunk.entities.remove(&entity) {
                        let id = *entity_ids.get(entity).unwrap();

                        old_inst.entity_removals.push(EntityRemoval {
                            pos: old_chunk_pos,
                            id,
                        });
                    }
                }
            }
        } else if old_loc.get() != loc.0 {
            // Entity changed the instance it's in. Remove it from old chunk and
            // insert it in the new chunk.

            if let Ok(mut old_inst) = instances.get_mut(old_loc.get()) {
                if let Some(old_chunk) = old_inst.chunk_mut(old_chunk_pos) {
                    if old_chunk.entities.remove(&entity) && old_chunk.is_viewed_mut() {
                        let id = *entity_ids.get(entity).unwrap();

                        old_inst.entity_removals.push(EntityRemoval {
                            pos: old_chunk_pos,
                            id,
                        });
                    }
                }
            }

            if let Ok(mut inst) = instances.get_mut(loc.0) {
                if let Some(chunk) = inst.chunk_mut(chunk_pos) {
                    if chunk.entities.insert(entity) && chunk.is_viewed_mut() {
                        inst.message_buf
                            .send(MessageCondition::View { pos: chunk_pos }, |w| {
                                init_item.write_init_packets(old_pos.get(), w)
                            });
                    }
                } else {
                    // Entity is now orphaned.
                    commands.entity(entity).insert(Orphaned);
                }
            }
        } else if chunk_pos != old_chunk_pos {
            // Entity changed its chunk position without changing instances. Remove it from
            // the old chunk and insert it in the new chunk.

            if let Ok(mut inst) = instances.get_mut(loc.0) {
                // TODO: extra hashmap lookup that isn't strictly necessary.
                let in_new_chunk = inst.chunk(chunk_pos).is_some();
                let mut in_old_chunk = true;

                if let Some(old_chunk) = inst.chunk_mut(old_chunk_pos) {
                    if old_chunk.entities.remove(&entity) && old_chunk.is_viewed_mut() {
                        let id = *entity_ids.get(entity).unwrap();

                        if in_new_chunk {
                            inst.message_buf.send_packet(
                                MessageCondition::TransitionView {
                                    viewed: old_chunk_pos,
                                    unviewed: chunk_pos,
                                },
                                &EntitiesDestroyS2c {
                                    entity_ids: Cow::Borrowed(&[id.get().into()]),
                                },
                            );
                        } else {
                            inst.entity_removals.push(EntityRemoval {
                                pos: old_chunk_pos,
                                id,
                            })
                        }
                    } else {
                        in_old_chunk = false;
                    }
                }

                if let Some(chunk) = inst.chunk_mut(chunk_pos) {
                    if chunk.entities.insert(entity) && chunk.is_viewed_mut() {
                        if in_old_chunk {
                            inst.message_buf.send(
                                MessageCondition::TransitionView {
                                    viewed: chunk_pos,
                                    unviewed: old_chunk_pos,
                                },
                                |w| init_item.write_init_packets(old_pos.get(), w),
                            );
                        } else {
                            inst.message_buf
                                .send(MessageCondition::View { pos: chunk_pos }, |w| {
                                    init_item.write_init_packets(old_pos.get(), w)
                                });
                        };
                    }
                } else {
                    // Entity is now orphaned.
                    commands.entity(entity).insert(Orphaned);
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
fn write_update_packets_per_chunk(
    mut instances: Query<&mut Instance>,
    mut entities: Query<UpdateEntityQuery, (With<EntityKind>, Without<Despawned>)>,
) {
    for inst in &mut instances {
        let inst = inst.into_inner();

        for (&pos, chunk) in &mut inst.chunks {
            chunk.update_pre_client(
                pos,
                &inst.info,
                &mut inst.message_buf,
                &mut inst.biome_changes,
                &mut entities,
            );
        }
    }
}

/// Clears changes made to instances and removes removed chunks.
fn update_post_client(mut instances: Query<&mut Instance>, mut commands: Commands) {
    for mut inst in &mut instances {
        inst.retain_chunks(|_, chunk| match chunk.state() {
            ChunkState::Removed | ChunkState::AddedRemoved => {
                // Any entities still in this chunk are now orphaned.
                for &entity in &chunk.entities {
                    if let Some(mut commands) = commands.get_entity(entity) {
                        commands.insert(Orphaned);
                    }
                }
                false
            }
            ChunkState::Added | ChunkState::Overwrite | ChunkState::Normal => {
                chunk.update_post_client();
                true
            }
        });

        inst.message_buf.clear();
        inst.entity_removals.clear();
        inst.biome_changes.clear();
    }
}
