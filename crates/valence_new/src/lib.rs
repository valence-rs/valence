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
#[derive(Copy, Clone, Component)]
pub struct Deleted;

const LIBRARY_NAMESPACE: &str = "valence";
