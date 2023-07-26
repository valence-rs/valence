#![doc = include_str!("../README.md")]
#![deny(
    rustdoc::broken_intra_doc_links,
    rustdoc::private_intra_doc_links,
    rustdoc::missing_crate_level_docs,
    rustdoc::invalid_codeblock_attributes,
    rustdoc::invalid_rust_codeblocks,
    rustdoc::bare_urls,
    rustdoc::invalid_html_tags
)]
#![warn(
    trivial_casts,
    trivial_numeric_casts,
    unused_lifetimes,
    unused_import_braces,
    unreachable_pub,
    clippy::dbg_macro
)]
#![allow(clippy::type_complexity)]

pub mod bvh;
pub mod chunk;
pub mod entity;
pub mod message;
pub mod packet;

use std::marker::PhantomData;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
pub use chunk::ChunkLayer;
pub use entity::EntityLayer;
use valence_biome::BiomeRegistry;
use valence_core::block_pos::BlockPos;
use valence_core::chunk_pos::ChunkPos;
use valence_core::ident::Ident;
use valence_core::protocol::encode::WritePacket;
use valence_core::Server;
use valence_dimension::DimensionTypeRegistry;
use valence_entity::{InitEntitiesSet, UpdateTrackedDataSet};

// Plugin is generic over the client type for hacky reasons.
pub struct LayerPlugin<Client: Component>(PhantomData<Client>);

impl<Client: Component> LayerPlugin<Client> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<Client: Component> Default for LayerPlugin<Client> {
    fn default() -> Self {
        Self::new()
    }
}

/// When entity and chunk changes are written to layers. Systems that modify
/// chunks and entities should run _before_ this. Systems that need to read
/// layer messages should run _after_ this.
#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct UpdateLayersPreClientSet;

/// When layers are cleared and messages from this tick are lost. Systems that
/// read layer messages should run _before_ this.
#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct UpdateLayersPostClientSet;

impl<Client: Component> Plugin for LayerPlugin<Client> {
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
        entity::build::<Client>(app);
    }
}

pub trait Layer: WritePacket {
    type ExceptWriter<'a>: WritePacket
    where
        Self: 'a;

    type ViewWriter<'a>: WritePacket
    where
        Self: 'a;

    type ViewExceptWriter<'a>: WritePacket
    where
        Self: 'a;

    type RadiusWriter<'a>: WritePacket
    where
        Self: 'a;

    type RadiusExceptWriter<'a>: WritePacket
    where
        Self: 'a;

    /// Returns a packet writer which sends packet data to all clients viewing
    /// the layer, except the client identified by `except`.
    fn except_writer(&mut self, except: Entity) -> Self::ExceptWriter<'_>;

    /// When writing packets to the view writer, only clients in view of `pos`
    /// will receive the packet.
    fn view_writer(&mut self, pos: impl Into<ChunkPos>) -> Self::ViewWriter<'_>;

    /// Like [`view_writer`](Self::view_writer), but packets written to the
    /// returned [`ViewExceptWriter`](Self::ViewExceptWriter) are not sent to
    /// the client identified by `except`.
    fn view_except_writer(
        &mut self,
        pos: impl Into<ChunkPos>,
        except: Entity,
    ) -> Self::ViewExceptWriter<'_>;

    fn radius_writer(&mut self, pos: impl Into<BlockPos>, radius: u32) -> Self::RadiusWriter<'_>;

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
    pub fn new(
        dimension_type_name: impl Into<Ident<String>>,
        dimensions: &DimensionTypeRegistry,
        biomes: &BiomeRegistry,
        server: &Server,
    ) -> Self {
        Self {
            chunk: ChunkLayer::new(dimension_type_name, dimensions, biomes, server),
            entity: EntityLayer::new(server),
        }
    }
}
