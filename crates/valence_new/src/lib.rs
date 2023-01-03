use bevy_ecs::component::Component;
pub use server::run_server;
pub use {anyhow, bevy_ecs as ecs};

pub mod biome;
pub mod client;
pub mod config;
pub mod dimension;
mod packet;
pub mod player_textures;
pub mod server;

/// A [`Component`] for marking entities as deleted.
///
/// In Valence, some built-in components such as ... are not allowed to be
/// removed from the [`World`] directly. Instead, you must give the entities you
/// wish to despawn the `Deleted` component. At the end of the tick, Valence
/// will despawn all entities with this component.
///
/// It is legal to remove components or delete entities that Valence does not
/// know about at any time.
#[derive(Copy, Clone, Component)]
pub struct Deleted;

const LIBRARY_NAMESPACE: &str = "valence";
