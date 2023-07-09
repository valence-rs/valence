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
use valence_core::protocol::encode::{PacketWriter, WritePacket};
use valence_core::protocol::{Encode, Packet};
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

pub trait Layer {
    type Global;
    type Local;

    fn send_global(&mut self, msg: Self::Global, f: impl FnOnce(&mut Vec<u8>));

    fn send_local(&mut self, msg: Self::Local, f: impl FnOnce(&mut Vec<u8>));

    fn compression_threshold(&self) -> Option<u32>;

    fn send_global_bytes(&mut self, msg: Self::Global, bytes: &[u8]) {
        self.send_global(msg, |b| b.extend_from_slice(bytes));
    }

    fn send_local_bytes(&mut self, msg: Self::Local, bytes: &[u8]) {
        self.send_local(msg, |b| b.extend_from_slice(bytes));
    }

    fn send_global_packet<P>(&mut self, msg: Self::Global, pkt: &P)
    where
        P: Encode + Packet,
    {
        let threshold = self.compression_threshold();

        self.send_global(msg, |b| PacketWriter::new(b, threshold).write_packet(pkt));
    }

    fn send_local_packet<P>(&mut self, msg: Self::Local, pkt: &P)
    where
        P: Encode + Packet,
    {
        let threshold = self.compression_threshold();

        self.send_local(msg, |b| PacketWriter::new(b, threshold).write_packet(pkt));
    }
}
