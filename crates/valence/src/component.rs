use std::fmt;
use std::ops::Deref;

use bevy_app::{CoreSet, Plugin};
/// Contains shared components and world queries.
use bevy_ecs::prelude::*;
use glam::{DVec3, Vec3};
use uuid::Uuid;
use valence_protocol::types::{GameMode as ProtocolGameMode, Property};

use crate::client::FlushPacketsSet;
use crate::util::{from_yaw_and_pitch, is_valid_username, to_yaw_and_pitch};
use crate::view::ChunkPos;

pub(crate) struct ComponentPlugin;

impl Plugin for ComponentPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(
            (update_old_position, update_old_location)
                .in_base_set(CoreSet::PostUpdate)
                .after(FlushPacketsSet),
        )
        .add_system(despawn_marked_entities.in_base_set(CoreSet::Last));
    }
}
