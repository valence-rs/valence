use bevy_ecs::prelude::*;

#[derive(Component, Copy, Clone, PartialEq, Eq, Debug)]
pub struct LayerId(pub Entity);

impl PartialEq<Entity> for LayerId {
    fn eq(&self, other: &Entity) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<LayerId> for Entity {
    fn eq(&self, other: &LayerId) -> bool {
        self.eq(&other.0)
    }
}
