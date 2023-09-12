use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::query::Has;
use valence_entity::{OldPosition, Position};
use valence_protocol::packets::play::{ChunkRenderDistanceCenterS2c, UnloadChunkS2c};
use valence_protocol::{ChunkPos, WritePacket};
use valence_server_common::Despawned;

use super::ChunkIndex;
use crate::client::{Client, ClientMarker, OldView, View};
use crate::layer::{LayerViewers, OldVisibleLayers, VisibleLayers};
use crate::ChunkView;

pub struct DimensionLayerPlugin;

/// When dimension layers are updated. This is where chunk packets are sent to
/// clients and chunk viewer counts are updated as client views change.
#[derive(SystemSet, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct UpdateDimensionLayerSet;

impl Plugin for DimensionLayerPlugin {
    fn build(&self, app: &mut App) {
        todo!()
    }
}

#[derive(SystemSet, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct UpdateEntityLayers;

#[derive(Event, Copy, Clone, PartialEq, Eq, Debug)]
pub struct ViewChunkEvent {
    pub client: Entity,
    pub pos: ChunkPos,
}

#[derive(Event, Copy, Clone, PartialEq, Eq, Debug)]
pub struct UnviewChunkEvent {
    pub client: Entity,
    pub pos: ChunkPos,
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
    mut layers: Query<&mut ChunkIndex>,
    mut view_events: EventWriter<ViewChunkEvent>,
    mut unview_events: EventWriter<UnviewChunkEvent>,
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

        let mut changed_dimension = false;

        if visible.is_changed() {
            // Send despawn packets for chunks in the old dimension layer.
            for &layer in old_visible.difference(&visible) {
                if let Ok(mut index) = layers.get_mut(layer) {
                    for pos in old_view.iter() {
                        if let Some(mut chunk) = index.get_mut(pos) {
                            client.write_packet(&UnloadChunkS2c { pos });
                            chunk.viewer_count -= 1;
                            unview_events.send(UnviewChunkEvent {
                                client: client_id,
                                pos,
                            });
                        }
                    }

                    changed_dimension = true;
                    break;
                }
            }

            // Send spawn packets for chunks in the new layer.
            for &layer in visible.difference(&old_visible) {
                if let Ok(mut index) = layers.get(layer) {
                    for pos in view.iter() {
                        if let Some(mut chunk) = index.get_mut(pos) {
                            chunk.write_chunk_init_packet(&mut *client, pos, index.info());
                            chunk.viewer_count += 1;
                            view_events.send(ViewChunkEvent {
                                client: client_id,
                                pos,
                            });
                        }
                    }

                    changed_dimension = true;
                    break;
                }
            }
        }

        if !changed_dimension && old_view != view {
            for &layer in visible.iter() {
                if let Ok(mut index) = layers.get_mut(layer) {
                    // Unload old chunks in view.
                    for pos in old_view.diff(view) {
                        if let Some(mut chunk) = index.get_mut(pos) {
                            client.write_packet(&UnloadChunkS2c { pos });
                            chunk.viewer_count -= 1;
                            unview_events.send(UnviewChunkEvent {
                                client: client_id,
                                pos,
                            });
                        }
                    }

                    // Load new chunks in view.
                    for pos in view.diff(old_view) {
                        if let Some(mut chunk) = index.get_mut(pos) {
                            chunk.write_chunk_init_packet(&mut *client, pos, index.info());
                            chunk.viewer_count += 1;
                            view_events.send(ViewChunkEvent {
                                client: client_id,
                                pos,
                            });
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
    mut unview_events: EventWriter<UnviewChunkEvent>,
) {
    for (client_id, old_view, old_layers) in &mut clients {
        for &layer in old_layers.iter() {
            if let Ok(mut chunks) = chunks.get_mut(layer) {
                for pos in old_view.get().iter() {
                    if let Some(chunk) = chunks.get_mut(pos) {
                        chunk.viewer_count -= 1;
                        unview_events.send(UnviewChunkEvent {
                            client: client_id,
                            pos,
                        });
                    }
                }

                break;
            }
        }
    }
}
