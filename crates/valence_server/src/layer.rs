mod chunk_view_index;
pub mod message;

use std::collections::BTreeSet;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::query::Has;
pub use chunk_view_index::ChunkViewIndex;
use derive_more::{Deref, DerefMut};
use valence_entity::{OldPosition, Position};
use valence_protocol::{ChunkPos, WritePacket};
use valence_registry::dimension_type::DimensionTypeId;
use valence_registry::{BiomeRegistry, DimensionTypeRegistry};
use valence_server_common::{Despawned, Server};

use self::message::{LayerMessages, MessageKind};
use crate::client::FlushPacketsSet;
use crate::dimension_layer::batch::Batch;
use crate::dimension_layer::{ChunkIndex, DimensionInfo};
use crate::layer::message::MessageScope;
use crate::{Client, DimensionLayerBundle};

/// Enables core functionality for layers.
pub struct LayerPlugin;

/// When queued messages in layers are written to the [`Client`] packet buffer
/// of all viewers.
#[derive(SystemSet, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct BroadcastLayerMessagesSet;

impl Plugin for LayerPlugin {
    fn build(&self, app: &mut App) {
        app.configure_set(
            PostUpdate,
            BroadcastLayerMessagesSet.before(FlushPacketsSet),
        )
        .add_systems(
            PostUpdate,
            (
                update_view_index,
                update_old_visible_layers,
                broadcast_layer_messages,
            )
                .chain()
                .in_set(BroadcastLayerMessagesSet),
        );
    }
}

/// Combination of components from [`DimensionLayerBundle`] and
/// [`EntityLayerBundle`]. The spawned entity from this bundle is considered
/// both a "dimension layer" and an "entity layer".
#[derive(Bundle)]
pub struct CombinedLayerBundle {
    pub chunk_index: ChunkIndex,
    pub block_batch: Batch,
    pub dimension_info: DimensionInfo,
    // TODO: entity layer components.
    pub chunk_view_index: ChunkViewIndex,
    pub layer_viewers: LayerViewers,
    pub layer_messages: LayerMessages,
}

impl CombinedLayerBundle {
    pub fn new(
        dimension_type: DimensionTypeId,
        dimensions: &DimensionTypeRegistry,
        biomes: &BiomeRegistry,
        server: &Server,
    ) -> Self {
        let DimensionLayerBundle {
            chunk_index,
            block_batch,
            dimension_info,
            chunk_view_index,
            layer_viewers,
            layer_messages,
        } = DimensionLayerBundle::new(dimension_type, dimensions, biomes, server);

        Self {
            chunk_index,
            block_batch,
            dimension_info,
            chunk_view_index,
            layer_viewers,
            layer_messages,
        }
    }
}

/// The set of layers a client is viewing.
#[derive(Component, Clone, Default, DerefMut, Deref, Debug)]
pub struct VisibleLayers(pub BTreeSet<Entity>);

/// The contents of [`VisibleLayers`] from the previous tick.
#[derive(Component, Default, Deref, Debug)]
pub struct OldVisibleLayers(BTreeSet<Entity>);

/// The set of clients that are viewing a layer.
///
/// This is updated automatically at the same time as [`ChunkViewIndex`].
#[derive(Component, Clone, Default, Deref, Debug)]
pub struct LayerViewers(BTreeSet<Entity>);

fn update_view_index(
    clients: Query<(
        Entity,
        Has<Despawned>,
        &OldPosition,
        Ref<Position>,
        &OldVisibleLayers,
        Ref<VisibleLayers>,
    )>,
    mut layers: Query<(&mut LayerViewers, &mut ChunkViewIndex)>,
) {
    for (client, is_despawned, old_pos, pos, old_visible, visible) in &clients {
        if is_despawned {
            // Remove from old layers.
            for &layer in old_visible.iter() {
                if let Ok((mut viewers, mut index)) = layers.get_mut(layer) {
                    let removed = viewers.0.remove(&client);
                    debug_assert!(removed);

                    let removed = index.remove(old_pos.get(), client);
                    debug_assert!(removed);
                }
            }
        } else if visible.is_changed() {
            // Remove from old layers.
            for &layer in old_visible.iter() {
                if let Ok((mut viewers, mut index)) = layers.get_mut(layer) {
                    let removed = viewers.0.remove(&client);
                    debug_assert!(removed);

                    let removed = index.remove(old_pos.get(), client);
                    debug_assert!(removed);
                }
            }

            // Insert in new layers.
            for &layer in visible.iter() {
                if let Ok((mut viewers, mut index)) = layers.get_mut(layer) {
                    let inserted = viewers.0.insert(client);
                    debug_assert!(inserted);

                    let inserted = index.insert(pos.0, client);
                    debug_assert!(inserted);
                }
            }
        } else if pos.is_changed() {
            // Change chunk cell in layers.

            let old_pos = ChunkPos::from(old_pos.get());
            let pos = ChunkPos::from(pos.0);

            if old_pos != pos {
                for &layer in visible.iter() {
                    if let Ok((_, mut index)) = layers.get_mut(layer) {
                        let removed = index.remove(old_pos, client);
                        debug_assert!(removed);

                        let inserted = index.insert(pos, client);
                        debug_assert!(inserted);
                    }
                }
            }
        }
    }
}

fn update_old_visible_layers(
    mut layers: Query<(&mut OldVisibleLayers, &VisibleLayers), Changed<VisibleLayers>>,
) {
    for (mut old, new) in &mut layers {
        old.0.clone_from(&new.0);
    }
}

fn broadcast_layer_messages(
    mut layers: Query<(&mut LayerMessages, &LayerViewers, &ChunkViewIndex)>,
    mut clients: Query<&mut Client>,
) {
    for (mut messages, viewers, view_index) in &mut layers {
        let mut acc = 0;

        for (scope, kind) in messages.messages() {
            let mut send = |client: Entity| {
                if let Ok(mut client) = clients.get_mut(client) {
                    match kind {
                        MessageKind::Packet { len } => {
                            client.write_packet_bytes(&messages.bytes()[acc..acc + len]);
                        }
                        MessageKind::EntityDespawn { entity } => todo!(),
                    }
                }
            };

            match scope {
                MessageScope::All => viewers.iter().copied().for_each(send),
                MessageScope::Only { only } => send(only),
                MessageScope::Except { except } => viewers
                    .iter()
                    .copied()
                    .filter(|&c| c != except)
                    .for_each(send),
                MessageScope::ChunkView { pos } => view_index.get(pos).for_each(send),
                MessageScope::ChunkViewExcept { pos, except } => {
                    view_index.get(pos).filter(|&e| e != except).for_each(send)
                }
                MessageScope::TransitionChunkView { include, exclude } => todo!(),
            }

            if let MessageKind::Packet { len } = kind {
                acc += len;
            }
        }

        messages.clear();
    }
}
