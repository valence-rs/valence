use bevy_ecs::prelude::*;
pub use {anyhow, bevy_app, bevy_ecs, valence_nbt as nbt, valence_protocol as protocol};

pub mod biome;
pub mod chunk_pos;
pub mod client;
pub mod config;
pub mod dimension;
pub mod entity;
pub mod instance;
pub mod inventory;
pub mod math;
mod packet;
pub mod player_list;
pub mod player_textures;
pub mod server;

/// A [`Component`] for marking entities that should be despawned at the end of
/// the tick.
///
/// In Valence, some built-in components such as [`McEntity`] are not allowed to
/// be removed from the [`World`] directly. Instead, you must give the entities
/// you wish to despawn the `Despawned` component. At the end of the tick,
/// Valence will despawn all entities with this component for you.
///
/// It is legal to remove components or delete entities that Valence does not
/// know about at any time.
///
/// [`McEntity`]: crate::entity::McEntity
#[derive(Copy, Clone, Component)]
pub struct Despawned;

pub type VisLevel = i16;

const LIBRARY_NAMESPACE: &str = "valence";

/// Let's pretend that [`NULL_ENTITY`] was created by spawning an entity,
/// immediately despawning it, and then stealing its [`Entity`] ID. The user
/// doesn't need to know about this.
const NULL_ENTITY: Entity = Entity::from_bits(u64::MAX);
