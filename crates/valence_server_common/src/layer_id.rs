use bevy_ecs::prelude::*;

/// The pointer to the layer this entity is a member of.
#[derive(Component, Copy, Clone, PartialEq, Eq, Debug)]
pub struct LayerId(pub Entity);

impl Default for LayerId {
    fn default() -> Self {
        Self(Entity::PLACEHOLDER)
    }
}

impl PartialEq<OldLayerId> for LayerId {
    fn eq(&self, other: &OldLayerId) -> bool {
        self.0 == other.0
    }
}

/// Value of [`LayerId`] from the previous tick. Not intended to be modified
/// manually.
#[derive(Component, Copy, Clone, PartialEq, Eq, Debug)]
pub struct OldLayerId(Entity);

impl OldLayerId {
    pub fn get(&self) -> Entity {
        self.0
    }
}

impl Default for OldLayerId {
    fn default() -> Self {
        Self(Entity::PLACEHOLDER)
    }
}

impl PartialEq<LayerId> for OldLayerId {
    fn eq(&self, other: &LayerId) -> bool {
        self.0 == other.0
    }
}

pub(super) fn update_old_layer_id(
    mut entities: Query<(&LayerId, &mut OldLayerId), Changed<LayerId>>,
) {
    for (new, mut old) in &mut entities {
        old.0 = new.0
    }
}
