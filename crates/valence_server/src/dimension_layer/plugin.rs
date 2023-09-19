use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_entity::Position;
use valence_protocol::packets::play::{
    ChunkLoadDistanceS2c, ChunkRenderDistanceCenterS2c, UnloadChunkS2c,
};
use valence_protocol::{VarInt, WritePacket};
use valence_server_common::Despawned;

use super::{ChunkIndex, DimensionInfo};
use crate::client::{Client, ClientMarker, OldView, View};
use crate::layer::{BroadcastLayerMessagesSet, OldVisibleLayers, VisibleLayers};

pub struct DimensionLayerPlugin;

/// When dimension layers are updated. This is where chunk packets are sent to
/// clients and chunk viewer counts are updated as client views change.
#[derive(SystemSet, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct UpdateDimensionLayerSet;

impl Plugin for DimensionLayerPlugin {
    fn build(&self, app: &mut App) {
        app.configure_set(
            PostUpdate,
            UpdateDimensionLayerSet.before(BroadcastLayerMessagesSet),
        )
        .add_systems(
            PostUpdate,
            (
                update_dimension_layer_views,
                update_dimension_layer_views_client_despawn,
            )
                .chain()
                .in_set(UpdateDimensionLayerSet),
        );
    }
}

fn update_dimension_layer_views(
    mut clients: Query<
        (
            Entity,
            &mut Client,
            OldView,
            View,
            &OldVisibleLayers,
            Ref<VisibleLayers>,
        ),
        Changed<Position>,
    >,
    mut layers: Query<(&mut ChunkIndex, &DimensionInfo)>,
) {
    for (client_id, mut client, old_view, view, old_visible, visible) in &mut clients {
        let old_view = old_view.get();
        let view = view.get();

        // Set render distance center before loading new chunks. Otherwise, client may
        // ignore them.
        if old_view.pos != view.pos {
            client.write_packet(&ChunkRenderDistanceCenterS2c {
                chunk_x: view.pos.x.into(),
                chunk_z: view.pos.z.into(),
            });
        }

        // Update view distance fog.
        // Note: this is just aesthetic.
        if old_view.dist() != view.dist() {
            client.write_packet(&ChunkLoadDistanceS2c {
                view_distance: VarInt(view.dist().into()),
            });
        }

        let mut changed_dimension = false;

        if visible.is_changed() {
            // Send despawn packets for chunks in the old dimension layer.
            for &layer in old_visible.difference(&visible) {
                if let Ok((mut index, _)) = layers.get_mut(layer) {
                    for pos in old_view.iter() {
                        if let Some(chunk) = index.get_mut(pos) {
                            client.write_packet(&UnloadChunkS2c { pos });
                            chunk.viewer_count -= 1;
                        }
                    }

                    changed_dimension = true;
                    break;
                }
            }

            // Send spawn packets for chunks in the new layer.
            for &layer in visible.difference(&old_visible) {
                if let Ok((mut index, info)) = layers.get_mut(layer) {
                    for pos in view.iter() {
                        if let Some(chunk) = index.get_mut(pos) {
                            chunk.write_chunk_init_packet(&mut *client, pos, info);
                            chunk.viewer_count += 1;
                        }
                    }

                    changed_dimension = true;
                    break;
                }
            }
        }

        if !changed_dimension && old_view != view {
            for &layer in visible.iter() {
                if let Ok((mut index, info)) = layers.get_mut(layer) {
                    // Unload old chunks in view.
                    for pos in old_view.diff(view) {
                        if let Some(chunk) = index.get_mut(pos) {
                            client.write_packet(&UnloadChunkS2c { pos });
                            chunk.viewer_count -= 1;
                        }
                    }

                    // Load new chunks in view.
                    for pos in view.diff(old_view) {
                        if let Some(chunk) = index.get_mut(pos) {
                            chunk.write_chunk_init_packet(&mut *client, pos, info);
                            chunk.viewer_count += 1;
                        }
                    }

                    break;
                }
            }
        }
    }
}

fn update_dimension_layer_views_client_despawn(
    mut clients: Query<(Entity, OldView, &OldVisibleLayers), (With<Despawned>, With<ClientMarker>)>,
    mut chunks: Query<&mut ChunkIndex>,
) {
    for (client_id, old_view, old_layers) in &mut clients {
        for &layer in old_layers.iter() {
            if let Ok(mut chunks) = chunks.get_mut(layer) {
                for pos in old_view.get().iter() {
                    if let Some(chunk) = chunks.get_mut(pos) {
                        chunk.viewer_count -= 1;
                    }
                }

                break;
            }
        }
    }
}
