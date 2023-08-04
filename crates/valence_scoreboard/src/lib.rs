#![doc = include_str!("../README.md")]

use std::collections::BTreeSet;

use bevy_app::prelude::*;
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::prelude::*;
use tracing::{debug, warn};

mod components;
pub use components::*;
use valence_client::{Client, OldVisibleEntityLayers, VisibleEntityLayers};
use valence_core::__private::VarInt;
use valence_core::despawn::Despawned;
use valence_core::text::IntoText;
use valence_entity::EntityLayerId;
use valence_layer::EntityLayer;
pub use valence_packet::packets::play::scoreboard_display_s2c::ScoreboardPosition;
use valence_packet::packets::play::scoreboard_display_s2c::*;
pub use valence_packet::packets::play::scoreboard_objective_update_s2c::ObjectiveRenderType;
use valence_packet::packets::play::scoreboard_objective_update_s2c::*;
use valence_packet::packets::play::scoreboard_player_update_s2c::*;
use valence_packet::protocol::encode::WritePacket;

/// Provides all necessary systems to manage scoreboards.
pub struct ScoreboardPlugin;

impl Plugin for ScoreboardPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (create_or_update_objectives, display_objectives),
        )
        .add_systems(PostUpdate, remove_despawned_objectives)
        .add_systems(PostUpdate, handle_new_clients)
        .add_systems(PostUpdate, update_scores.after(create_or_update_objectives));
    }
}

fn create_or_update_objectives(
    objectives: Query<
        (
            Ref<Objective>,
            &ObjectiveDisplay,
            &ObjectiveRenderType,
            &EntityLayerId,
        ),
        Or<(Changed<ObjectiveDisplay>, Changed<ObjectiveRenderType>)>,
    >,
    mut layers: Query<&mut EntityLayer>,
) {
    for (objective, display, render_type, entity_layer) in objectives.iter() {
        let mode = if objective.is_added() {
            ObjectiveMode::Create {
                objective_display_name: (&display.0).into_cow_text(),
                render_type: *render_type,
            }
        } else {
            ObjectiveMode::Update {
                objective_display_name: (&display.0).into_cow_text(),
                render_type: *render_type,
            }
        };

        let Ok(mut layer) = layers.get_mut(entity_layer.0) else {
            warn!("No layer found for entity layer ID {:?}, can't update scoreboard objective", entity_layer);
            continue;
        };

        layer.write_packet(&ScoreboardObjectiveUpdateS2c {
            objective_name: &objective.0,
            mode,
        });
    }
}

/// Must occur after `create_or_update_objectives`.
fn display_objectives(
    objectives: Query<
        (&Objective, Ref<ScoreboardPosition>, &EntityLayerId),
        Changed<ScoreboardPosition>,
    >,
    mut layers: Query<&mut EntityLayer>,
) {
    for (objective, position, entity_layer) in objectives.iter() {
        let packet = ScoreboardDisplayS2c {
            score_name: &objective.0,
            position: *position,
        };

        let Ok(mut layer) = layers.get_mut(entity_layer.0) else {
            warn!("No layer found for entity layer ID {:?}, can't update scoreboard display", entity_layer);
            continue;
        };

        layer.write_packet(&packet);
    }
}

fn remove_despawned_objectives(
    mut commands: Commands,
    objectives: Query<(Entity, &Objective, &EntityLayerId), With<Despawned>>,
    mut layers: Query<&mut EntityLayer>,
) {
    for (entity, objective, entity_layer) in objectives.iter() {
        commands.entity(entity).despawn();
        let Ok(mut layer) = layers.get_mut(entity_layer.0) else {
            warn!("No layer found for entity layer ID {:?}, can't remove scoreboard objective", entity_layer);
            continue;
        };

        layer.write_packet(&ScoreboardObjectiveUpdateS2c {
            objective_name: &objective.0,
            mode: ObjectiveMode::Remove,
        });
    }
}

fn handle_new_clients(
    mut clients: Query<
        (&mut Client, &VisibleEntityLayers, &OldVisibleEntityLayers),
        Or<(Added<Client>, Changed<VisibleEntityLayers>)>,
    >,
    objectives: Query<
        (
            &Objective,
            &ObjectiveDisplay,
            &ObjectiveRenderType,
            &ScoreboardPosition,
            &ObjectiveScores,
            &EntityLayerId,
        ),
        Without<Despawned>,
    >,
) {
    // Remove objectives from the old visible layers that are not in the new visible
    // layers.
    for (mut client, visible_layers, old_visible_layers) in clients.iter_mut() {
        let removed_layers: BTreeSet<_> = old_visible_layers
            .get()
            .difference(&visible_layers.0)
            .collect();

        for (objective, _, _, _, _, layer) in objectives.iter() {
            if !removed_layers.contains(&layer.0) {
                continue;
            }
            client.write_packet(&ScoreboardObjectiveUpdateS2c {
                objective_name: &objective.0,
                mode: ObjectiveMode::Remove,
            });
        }
    }

    // Add objectives from the new visible layers that are not in the old visible
    // layers, or send all objectives if the client is new.
    for (mut client, visible_layers, old_visible_layers) in clients.iter_mut() {
        // not sure how to avoid the clone here
        let added_layers = if client.is_added() {
            debug!("client is new, sending all objectives");
            visible_layers.0.clone()
        } else {
            visible_layers
                .0
                .difference(&old_visible_layers.get())
                .copied()
                .collect::<BTreeSet<_>>()
        };

        for (objective, display, render_type, position, scores, layer) in objectives.iter() {
            if !added_layers.contains(&layer.0) {
                continue;
            }

            client.write_packet(&ScoreboardObjectiveUpdateS2c {
                objective_name: &objective.0,
                mode: ObjectiveMode::Create {
                    objective_display_name: (&display.0).into_cow_text(),
                    render_type: *render_type,
                },
            });
            client.write_packet(&ScoreboardDisplayS2c {
                score_name: &objective.0,
                position: *position,
            });

            for (uuid, score) in &scores.0 {
                let packet = ScoreboardPlayerUpdateS2c {
                    entity_name: &uuid.0.to_string(),
                    action: ScoreboardPlayerUpdateAction::Update {
                        objective_name: &objective.0,
                        objective_score: VarInt(*score),
                    },
                };

                client.write_packet(&packet);
            }
        }
    }
}

fn update_scores(
    objectives: Query<
        (&Objective, &ObjectiveScores, &EntityLayerId),
        (Changed<ObjectiveScores>, Without<Despawned>),
    >,
    mut layers: Query<&mut EntityLayer>,
) {
    for (objective, scores, entity_layer) in objectives.iter() {
        let Ok(mut layer) = layers.get_mut(entity_layer.0) else {
            warn!("No layer found for entity layer ID {:?}, can't update scores", entity_layer);
            continue;
        };

        // TODO: send only the difference between the old and new scores.
        for (uuid, score) in &scores.0 {
            let packet = ScoreboardPlayerUpdateS2c {
                entity_name: &uuid.0.to_string(),
                action: ScoreboardPlayerUpdateAction::Update {
                    objective_name: &objective.0,
                    objective_score: VarInt(*score),
                },
            };

            layer.write_packet(&packet);
        }
    }
}
