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

pub mod abilities;
pub mod action;
mod chunk_view;
pub mod client;
pub mod client_command;
pub mod client_settings;
pub mod custom_payload;
pub mod event_loop;
pub mod hand_swing;
pub mod interact_block;
pub mod interact_entity;
pub mod interact_item;
pub mod keepalive;
pub mod layer;
pub mod message;
pub mod movement;
pub mod op_level;
pub mod placement;
pub mod resource_pack;
pub mod spawn;
pub mod status;
pub mod teleport;
pub mod title;

pub use chunk_view::ChunkView;
pub use event_loop::{EventLoopPostUpdate, EventLoopPreUpdate, EventLoopUpdate};
pub use layer::{ChunkLayer, EntityLayer, Layer, LayerBundle};
pub use valence_protocol::{
    block, ident, item, math, text, uuid, BlockPos, BlockState, ChunkPos, CompressionThreshold,
    Difficulty, Direction, GameMode, Hand, Ident, ItemKind, ItemStack, Text, MINECRAFT_VERSION,
    PROTOCOL_VERSION,
};
pub use valence_server_common::*;
pub use {
    bevy_app as app, bevy_ecs as ecs, rand, valence_entity as entity, valence_nbt as nbt,
    valence_protocol as protocol, valence_registry as registry,
};
