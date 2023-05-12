use bevy_ecs::prelude::*;

/// A marker [`Component`] for entities that should be despawned at the end of
/// the tick.
///
/// In Valence, some entities such as Minecraft entities must not be removed
/// from the [`World`] directly. Valence needs an opportunity to perform
/// deinitialization work while the entity's components still exist.
///
/// To resolve this problem, you must give the entities you wish to despawn the
/// `Despawned` component. At the end of the tick, Valence will despawn all
/// entities with this component for you.
///
/// The `Despawned` component can be used on entities that Valence does not know
/// about. The entity will be despawned regardless.
#[derive(Component, Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct Despawned;

pub(super) fn despawn_marked_entities(
    mut commands: Commands,
    entities: Query<Entity, With<Despawned>>,
) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
}
