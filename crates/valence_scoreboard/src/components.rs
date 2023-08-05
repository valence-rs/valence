use std::collections::HashMap;

use bevy_ecs::prelude::*;
use valence_core::text::{IntoText, Text};
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
        let name = name.into();
        debug_assert!(
            name.len() <= 16,
            "Objective name {} is too long ({} > 16)",
            name,
            name.len()
        );
        Self(name)
    }

    pub fn name(&self) -> &str {
        &self.0
    }
}

/// Optional display name for an objective. If not present, the objective's name
/// is used.
#[derive(Debug, Clone, PartialEq, Component)]
pub struct ObjectiveDisplay(pub Text);

/// A mapping of keys to their scores.
#[derive(Debug, Clone, Component, Default)]
pub struct ObjectiveScores(pub(crate) HashMap<String, i32>);

impl ObjectiveScores {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_map(map: impl Into<HashMap<String, i32>>) -> Self {
        Self(map.into())
    }

    pub fn get(&self, key: &str) -> Option<&i32> {
        self.0.get(key)
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut i32> {
        self.0.get_mut(key)
    }

    pub fn insert(&mut self, key: impl Into<String>, value: i32) -> Option<i32> {
        self.0.insert(key.into(), value)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Component)]
pub struct OldObjectiveScores(pub(crate) HashMap<String, i32>);

impl OldObjectiveScores {
    pub fn diff<'a>(&'a self, scores: &'a ObjectiveScores) -> Vec<&'a str> {
        let mut diff = Vec::new();

        for (key, value) in &self.0 {
            if scores.0.get(key) != Some(value) {
                diff.push(key.as_str());
            }
        }

        let new_keys = scores
            .0
            .keys()
            .filter(|key| !self.0.contains_key(key.as_str()))
            .map(|key| key.as_str());

        let removed_keys = self
            .0
            .keys()
            .filter(|key| !scores.0.contains_key(key.as_str()))
            .map(|key| key.as_str());

        diff.extend(new_keys);
        diff.extend(removed_keys);
        diff
    }
}

#[derive(Bundle)]
pub struct ObjectiveBundle {
    pub name: Objective,
    pub display: ObjectiveDisplay,
    pub render_type: ObjectiveRenderType,
    pub scores: ObjectiveScores,
    pub old_scores: OldObjectiveScores,
    pub position: ScoreboardPosition,
    pub layer: EntityLayerId,
}

impl Default for ObjectiveBundle {
    fn default() -> Self {
        Self {
            name: Objective::new(""),
            display: ObjectiveDisplay("".into_text()),
            render_type: Default::default(),
            scores: Default::default(),
            old_scores: Default::default(),
            position: Default::default(),
            layer: Default::default(),
        }
    }
}
