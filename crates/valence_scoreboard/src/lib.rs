use bevy_app::prelude::*;
use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::prelude::*;
use tracing::warn;

mod components;
pub use components::*;
use valence_core::text::IntoText;
use valence_core::uuid::UniqueId;
use valence_entity::EntityLayerId;
use valence_layer::{EntityLayer, Layer};
pub use valence_packet::packets::play::scoreboard_objective_update_s2c::ObjectiveRenderType;
use valence_packet::packets::play::scoreboard_objective_update_s2c::*;
use valence_packet::protocol::encode::WritePacket;

/// Provides all necessary systems to manage scoreboards.
pub struct ScoreboardPlugin;

impl Plugin for ScoreboardPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, create_or_update_objectives);
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

        layer
            .write_packet_fallible(&ScoreboardObjectiveUpdateS2c {
                objective_name: &objective.0,
                mode,
            })
            .expect("Failed to write scoreboard objective update packet");
    }
}
