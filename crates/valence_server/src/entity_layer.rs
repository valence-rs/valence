use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

use crate::layer::BroadcastLayerMessagesSet;

pub struct EntityLayerPlugin;

/// When entity changes are written to entity layers and clients are sent
/// spawn/despawn packets.
#[derive(SystemSet, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct UpdateEntityLayerSet;

impl Plugin for EntityLayerPlugin {
    fn build(&self, app: &mut App) {
        app.configure_set(
            PostUpdate,
            UpdateEntityLayerSet.before(BroadcastLayerMessagesSet),
        );
    }
}
