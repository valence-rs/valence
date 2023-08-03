use std::collections::{HashMap, HashSet};

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use valence_core::text::Text;
use valence_core::uuid::UniqueId;

/// A string that identifies an objective. There is one scoreboard per
/// objective.It's generally not safe to modify this after it's been created.
/// Limited to 16 characters.
///
/// Directly analogous to an Objective's Name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Component)]
pub struct Objective(String);

impl Objective {
    pub fn new(name: String) -> Self {
        Self(name)
    }
}

/// Optional display name for an objective. If not present, the objective's name
/// is used.
#[derive(Debug, Clone, PartialEq, Component)]
pub struct ObjectiveDisplay(pub Text);

#[derive(Debug, Clone, PartialEq, Eq, Component, Default)]
pub struct ObjectiveEntities(HashSet<Entity>);

/// A mapping of entity UUIDs to their scores.
#[derive(Debug, Clone, Component, Default)]
pub struct ObjectiveScores(HashMap<UniqueId, i32>);

#[derive(Bundle)]
pub struct ObjectiveBundle {
    pub name: Objective,
    pub display: ObjectiveDisplay,
    pub entities: ObjectiveEntities,
    pub scores: ObjectiveScores,
}
