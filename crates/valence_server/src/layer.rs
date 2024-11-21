//! Defines chunk layers and entity layers. Chunk layers contain the chunks and
//! dimension data of a world, while entity layers contain all the Minecraft
//! entities.
//!
//! These two together are analogous to Minecraft "levels" or "worlds".

pub mod bvh;
pub mod chunk;
pub mod entity;
pub mod message;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
pub use chunk::ChunkLayer;
pub use entity::EntityLayer;
use valence_entity::{InitEntitiesSet, UpdateTrackedDataSet};
use valence_protocol::encode::WritePacket;
use valence_protocol::{BlockPos, ChunkPos, Ident};
use valence_registry::{BiomeRegistry, DimensionTypeRegistry};
use valence_server_common::Server;

pub struct LayerPlugin;

/// When entity and chunk changes are written to layers. Systems that modify
/// chunks and entities should run _before_ this. Systems that need to read
/// layer messages should run _after_ this.
#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct UpdateLayersPreClientSet;

/// When layers are cleared and messages from this tick are lost. Systems that
/// read layer messages should run _before_ this.
#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct UpdateLayersPostClientSet;

impl Plugin for LayerPlugin {
    fn build(&self, app: &mut App) {
        app.configure_sets(
            PostUpdate,
            (
                UpdateLayersPreClientSet
                    .after(InitEntitiesSet)
                    .after(UpdateTrackedDataSet),
                UpdateLayersPostClientSet.after(UpdateLayersPreClientSet),
            ),
        );

        chunk::build(app);
        entity::build(app);
    }
}

/// Common functionality for layers. Notable implementors are [`ChunkLayer`] and
/// [`EntityLayer`].
///
/// Layers support sending packets to viewers of the layer under various
/// conditions. These are the "packet writers" exposed by this trait.
///
/// Layers themselves implement the [`WritePacket`] trait. Writing directly to a
/// layer will send packets to all viewers unconditionally.
pub trait Layer: WritePacket {
    /// Packet writer returned by [`except_writer`](Self::except_writer).
    type ExceptWriter<'a>: WritePacket
    where
        Self: 'a;

    /// Packet writer returned by [`view_writer`](Self::ViewWriter).
    type ViewWriter<'a>: WritePacket
    where
        Self: 'a;

    /// Packet writer returned by
    /// [`view_except_writer`](Self::ViewExceptWriter).
    type ViewExceptWriter<'a>: WritePacket
    where
        Self: 'a;

    /// Packet writer returned by [`radius_writer`](Self::radius_writer).
    type RadiusWriter<'a>: WritePacket
    where
        Self: 'a;

    /// Packet writer returned by
    /// [`radius_except_writer`](Self::radius_except_writer).
    type RadiusExceptWriter<'a>: WritePacket
    where
        Self: 'a;

    /// Returns a packet writer which sends packets to all viewers not
    /// identified by `except`.
    fn except_writer(&mut self, except: Entity) -> Self::ExceptWriter<'_>;

    /// Returns a packet writer which sends packets to viewers in view of
    /// the chunk position `pos`.
    fn view_writer(&mut self, pos: impl Into<ChunkPos>) -> Self::ViewWriter<'_>;

    /// Returns a packet writer which sends packets to viewers in
    /// view of the chunk position `pos` and not identified by `except`.
    fn view_except_writer(
        &mut self,
        pos: impl Into<ChunkPos>,
        except: Entity,
    ) -> Self::ViewExceptWriter<'_>;

    /// Returns a packet writer which sends packets to viewers within `radius`
    /// blocks of the block position `pos`.
    fn radius_writer(&mut self, pos: impl Into<BlockPos>, radius: u32) -> Self::RadiusWriter<'_>;

    /// Returns a packet writer which sends packets to viewers within `radius`
    /// blocks of the block position `pos` and not identified by `except`.
    fn radius_except_writer(
        &mut self,
        pos: impl Into<BlockPos>,
        radius: u32,
        except: Entity,
    ) -> Self::RadiusExceptWriter<'_>;
}

/// Convenience [`Bundle`] for spawning a layer entity with both [`ChunkLayer`]
/// and [`EntityLayer`] components.
#[derive(Bundle)]
pub struct LayerBundle {
    pub chunk: ChunkLayer,
    pub entity: EntityLayer,
}

impl LayerBundle {
    /// Returns a new layer bundle.
    pub fn new(
        dimension_type: Ident<&str>,
        dimensions: &DimensionTypeRegistry,
        biomes: &BiomeRegistry,
        server: &Server,
    ) -> Self {
        Self {
            chunk: ChunkLayer::new(dimension_type, dimensions, biomes, server),
            entity: EntityLayer::new(server),
        }
    }
}
