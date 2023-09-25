use bevy_ecs::prelude::*;
use derive_more::Deref;
use uuid::Uuid;

/// The universally unique identifier of an entity. Component wrapper for a
/// [`Uuid`].
///
/// This component is expected to remain _unique_ and _constant_ during the
/// lifetime of the entity. The [`Default`] impl generates a new random UUID.
#[derive(Component, Copy, Clone, PartialEq, Eq, Debug, PartialOrd, Ord, Hash, Deref)]
pub struct UniqueId(pub Uuid);

/// Generates a new random UUID.
impl Default for UniqueId {
    fn default() -> Self {
        Self(Uuid::from_bytes(rand::random()))
    }
}
