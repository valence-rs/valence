use std::collections::HashMap;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_core::text::Text;
use valence_core::uuid::UniqueId;
use valence_entity::EntityLayerId;
use valence_packet::packets::play::scoreboard_display_s2c::ScoreboardPosition;
use valence_packet::packets::play::scoreboard_objective_update_s2c::ObjectiveRenderType;

/// A string that identifies an objective. There is one scoreboard per
/// objective.It's generally not safe to modify this after it's been created.
/// Limited to 16 characters.
///
/// Directly analogous to an Objective's Name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Component)]
pub struct Objective(pub(crate) String);

impl Objective {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }
}

/// Optional display name for an objective. If not present, the objective's name
/// is used.
#[derive(Debug, Clone, PartialEq, Component)]
pub struct ObjectiveDisplay(pub Text);

#[derive(Debug, Clone, Component, Default)]
pub enum ObjectiveValueType {
    /// Display the value as a number.
    #[default]
    Integer,
    /// Display the value as hearts.
    Hearts,
}

/// A mapping of entity UUIDs to their scores.
#[derive(Debug, Clone, Component, Default)]
pub struct ObjectiveScores(HashMap<UniqueId, i32>);

impl ObjectiveScores {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_map(map: impl Into<HashMap<UniqueId, i32>>) -> Self {
        Self(map.into())
    }
}

#[derive(Bundle)]
pub struct ObjectiveBundle {
    pub name: Objective,
    pub display: ObjectiveDisplay,
    pub render_type: ObjectiveRenderType,
    pub scores: ObjectiveScores,
    pub position: ScoreboardPosition,
    pub layer: EntityLayerId,
}
