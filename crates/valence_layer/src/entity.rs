use std::collections::hash_map::Entry;
use std::collections::BTreeSet;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::query::Has;
use rustc_hash::FxHashMap;
use valence_core::chunk_pos::ChunkPos;
use valence_core::despawn::Despawned;
use valence_core::protocol::encode::{PacketWriter, WritePacket};
use valence_core::protocol::{Encode, Packet};
use valence_core::Server;
use valence_entity::query::UpdateEntityQuery;
use valence_entity::{EntityId, EntityLayerId, OldEntityLayerId, OldPosition, Position};

use crate::bvh::GetChunkPos;
use crate::message::Messages;
use crate::{Layer, UpdateLayersPostClientSet, UpdateLayersPreClientSet};

#[derive(Component, Debug)]
pub struct EntityLayer {
    messages: EntityLayerMessages,
    entities: FxHashMap<ChunkPos, BTreeSet<Entity>>,
    compression_threshold: Option<u32>,
}

type EntityLayerMessages = Messages<GlobalMsg, LocalMsg>;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum GlobalMsg {
    /// Send packet data to all clients viewing the layer. Message data is
    /// serialized packet data.
    Packet,
    /// Send packet data to all clients viewing layer, except the client
    /// identified by `except`.
    PacketExcept { except: Entity },
    /// This layer was despawned and should be removed from the set of visible
    /// entity layers. Message data is empty.
    DespawnLayer,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum LocalMsg {
    /// Spawn entities if the client is not already viewing `src_layer`. Message
    /// data is the serialized form of [`Entity`].
    SpawnEntity { pos: ChunkPos, src_layer: Entity },
    /// Spawn entities if the client is not in view of `src_pos`. Message data
    /// is the serialized form of [`Entity`].
    SpawnEntityTransition { pos: ChunkPos, src_pos: ChunkPos },
    /// Send packet data to all clients viewing the layer in view of `pos`.
    /// Message data is serialized packet data.
    PacketAt { pos: ChunkPos },
    /// Send packet data to all clients viewing the layer in view of `pos`,
    /// except the client identified by `except`. Message data is serialized
    /// packet data.
    PacketAtExcept { pos: ChunkPos, except: Entity },
    /// Despawn entities if the client is not already viewing `dest_layer`.
    /// Message data is the serialized form of `EntityId`.
    DespawnEntity { pos: ChunkPos, dest_layer: Entity },
    /// Despawn entities if the client is not in view of `dest_pos`. Message
    /// data is the serialized form of `EntityId`.
    DespawnEntityTransition { pos: ChunkPos, dest_pos: ChunkPos },
}

impl GetChunkPos for LocalMsg {
    fn chunk_pos(&self) -> ChunkPos {
        match *self {
            LocalMsg::PacketAt { pos } => pos,
            LocalMsg::PacketAtExcept { pos, .. } => pos,
            LocalMsg::SpawnEntity { pos, .. } => pos,
            LocalMsg::SpawnEntityTransition { pos, .. } => pos,
            LocalMsg::DespawnEntity { pos, .. } => pos,
            LocalMsg::DespawnEntityTransition { pos, .. } => pos,
        }
    }
}

impl EntityLayer {
    pub fn new(server: &Server) -> Self {
        Self {
            messages: Messages::new(),
            entities: Default::default(),
            compression_threshold: server.compression_threshold(),
        }
    }

    /// Returns a list of entities with positions within the provided chunk
    /// position on this layer.
    pub fn entities_at(
        &self,
        pos: impl Into<ChunkPos>,
    ) -> impl Iterator<Item = Entity> + Clone + '_ {
        self.entities
            .get(&pos.into())
            .into_iter()
            .flat_map(|entities| entities.iter().copied())
    }

    #[doc(hidden)]
    pub fn messages(&self) -> &EntityLayerMessages {
        &self.messages
    }
}

impl Layer for EntityLayer {
    type Global = GlobalMsg;

    type Local = LocalMsg;

    fn send_global(&mut self, msg: Self::Global, f: impl FnOnce(&mut Vec<u8>)) {
        self.messages.send_global(msg, f);
    }

    fn send_local(&mut self, msg: Self::Local, f: impl FnOnce(&mut Vec<u8>)) {
        self.messages.send_local(msg, f);
    }

    fn compression_threshold(&self) -> Option<u32> {
        self.compression_threshold
    }
}

impl WritePacket for EntityLayer {
    fn write_packet_fallible<P>(&mut self, packet: &P) -> anyhow::Result<()>
    where
        P: Packet + Encode,
    {
        self.send_global_packet(GlobalMsg::Packet, packet);

        // TODO: propagate error up.
        Ok(())
    }

    fn write_packet_bytes(&mut self, bytes: &[u8]) {
        self.send_global_bytes(GlobalMsg::Packet, bytes)
    }
}

pub(super) fn build<Client: Component>(app: &mut App) {
    app.add_systems(
        PostUpdate,
        (
            (
                change_entity_positions,
                send_entity_update_messages::<Client>,
                send_layer_despawn_messages,
                ready_entity_layers,
            )
                .chain()
                .in_set(UpdateLayersPreClientSet),
            unready_entity_layers.in_set(UpdateLayersPostClientSet),
        ),
    );
}

fn change_entity_positions(
    entities: Query<
        (
            Entity,
            &EntityId,
            &Position,
            &OldPosition,
            &EntityLayerId,
            &OldEntityLayerId,
            Has<Despawned>,
        ),
        Or<(Changed<Position>, Changed<EntityLayerId>, With<Despawned>)>,
    >,
    mut layers: Query<&mut EntityLayer>,
) {
    for (entity, entity_id, pos, old_pos, layer_id, old_layer_id, despawned) in &entities {
        let chunk_pos = pos.chunk_pos();
        let old_chunk_pos = old_pos.chunk_pos();

        if despawned {
            // Entity was deleted. Remove it from the layer.

            if let Ok(old_layer) = layers.get_mut(layer_id.0) {
                let old_layer = old_layer.into_inner();

                if let Entry::Occupied(mut old_cell) = old_layer.entities.entry(old_chunk_pos) {
                    if old_cell.get_mut().remove(&entity) {
                        old_layer.messages.send_local(
                            LocalMsg::DespawnEntity {
                                pos: old_chunk_pos,
                                dest_layer: Entity::PLACEHOLDER,
                            },
                            |b| b.extend_from_slice(&entity_id.get().to_ne_bytes()),
                        );

                        if old_cell.get().is_empty() {
                            old_cell.remove();
                        }
                    }
                }
            }
        } else if old_layer_id != layer_id {
            // Entity changed their layer. Remove it from old layer and insert it in the new
            // layer.

            if let Ok(old_layer) = layers.get_mut(old_layer_id.get()) {
                let old_layer = old_layer.into_inner();

                if let Entry::Occupied(mut old_cell) = old_layer.entities.entry(old_chunk_pos) {
                    if old_cell.get_mut().remove(&entity) {
                        old_layer.messages.send_local(
                            LocalMsg::DespawnEntity {
                                pos: old_chunk_pos,
                                dest_layer: layer_id.0,
                            },
                            |b| b.extend_from_slice(&entity_id.get().to_ne_bytes()),
                        );

                        if old_cell.get().is_empty() {
                            old_cell.remove();
                        }
                    }
                }
            }

            if let Ok(mut layer) = layers.get_mut(layer_id.0) {
                if layer.entities.entry(chunk_pos).or_default().insert(entity) {
                    layer.messages.send_local(
                        LocalMsg::SpawnEntity {
                            pos: chunk_pos,
                            src_layer: old_layer_id.get(),
                        },
                        |b| b.extend_from_slice(&entity.to_bits().to_ne_bytes()),
                    );
                }
            }
        } else if chunk_pos != old_chunk_pos {
            // Entity changed their chunk position without changing layers. Remove it from
            // old cell and insert it in the new cell.

            if let Ok(mut layer) = layers.get_mut(layer_id.0) {
                if let Entry::Occupied(mut old_cell) = layer.entities.entry(old_chunk_pos) {
                    if old_cell.get_mut().remove(&entity) {
                        layer.messages.send_local(
                            LocalMsg::DespawnEntityTransition {
                                pos: old_chunk_pos,
                                dest_pos: chunk_pos,
                            },
                            |b| b.extend_from_slice(&entity_id.get().to_ne_bytes()),
                        );
                    }
                }

                if layer.entities.entry(chunk_pos).or_default().insert(entity) {
                    layer.messages.send_local(
                        LocalMsg::SpawnEntityTransition {
                            pos: chunk_pos,
                            src_pos: old_chunk_pos,
                        },
                        |b| b.extend_from_slice(&entity.to_bits().to_ne_bytes()),
                    );
                }
            }
        }
    }
}

fn send_entity_update_messages<Client: Component>(
    entities: Query<(Entity, UpdateEntityQuery, Has<Client>), Without<Despawned>>,
    mut layers: Query<&mut EntityLayer>,
) {
    for layer in layers.iter_mut() {
        let layer = layer.into_inner();

        for cell in layer.entities.values_mut() {
            for &entity in cell.iter() {
                if let Ok((entity, update, is_client)) = entities.get(entity) {
                    let chunk_pos = update.pos.chunk_pos();

                    // Send the update packets to all viewers. If the entity being updated is a
                    // client, then we need to be careful to exclude the client itself from
                    // receiving the update packets.
                    let msg = if is_client {
                        LocalMsg::PacketAtExcept {
                            pos: chunk_pos,
                            except: entity,
                        }
                    } else {
                        LocalMsg::PacketAt { pos: chunk_pos }
                    };

                    layer.messages.send_local(msg, |b| {
                        update
                            .write_update_packets(PacketWriter::new(b, layer.compression_threshold))
                    });
                } else {
                    panic!(
                        "Entity {entity:?} was not properly removed from entity layer. Did you \
                         forget to use the `Despawned` component?"
                    );
                }
            }
        }
    }
}

fn send_layer_despawn_messages(mut layers: Query<&mut EntityLayer, With<Despawned>>) {
    for mut layer in &mut layers {
        layer.send_global(GlobalMsg::DespawnLayer, |_| {});
    }
}

fn ready_entity_layers(mut layers: Query<&mut EntityLayer>) {
    for mut layer in &mut layers {
        layer.messages.ready();
    }
}

fn unready_entity_layers(mut layers: Query<&mut EntityLayer>) {
    for mut layer in &mut layers {
        layer.messages.unready();
    }
}
