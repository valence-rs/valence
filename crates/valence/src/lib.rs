#![doc = include_str!("../../../README.md")] // Points to the main project README.
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/valence-rs/valence/main/assets/logo.svg",
    html_favicon_url = "https://raw.githubusercontent.com/valence-rs/valence/main/assets/logo.svg"
)]
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

use bevy_app::{PluginGroup, PluginGroupBuilder};

#[cfg(test)]
mod tests;

#[cfg(feature = "advancement")]
pub use valence_advancement as advancement;
#[cfg(feature = "anvil")]
pub use valence_anvil as anvil;
pub use valence_core::*;
#[cfg(feature = "inventory")]
pub use valence_inventory as inventory;
#[cfg(feature = "network")]
pub use valence_network as network;
#[cfg(feature = "player_list")]
pub use valence_player_list as player_list;
#[cfg(feature = "world_border")]
pub use valence_world_border as world_border;
#[cfg(feature = "boss_bar")]
pub use valence_boss_bar as boss_bar;
pub use {
    bevy_app as app, bevy_ecs as ecs, glam, valence_biome as biome, valence_block as block,
    valence_client as client, valence_dimension as dimension, valence_entity as entity,
    valence_instance as instance, valence_nbt as nbt, valence_registry as registry,
};

/// Contains the most frequently used items in Valence projects.
///
/// This is usually glob imported like so:
///
/// ```
/// use valence::prelude::*; // Glob import.
///
/// let mut app = App::new();
/// app.add_system(|| println!("yippee!"));
/// // ...
/// ```
pub mod prelude {
    pub use ::uuid::Uuid;
    pub use app::prelude::*;
    pub use bevy_ecs; // Needed for bevy_ecs proc macros to function correctly.
    pub use biome::{Biome, BiomeId, BiomeRegistry};
    pub use block::{BlockKind, BlockState, PropName, PropValue};
    pub use block_pos::BlockPos;
    pub use chunk_pos::{ChunkPos, ChunkView};
    pub use client::action::*;
    pub use client::command::*;
    pub use client::event_loop::{EventLoopSchedule, EventLoopSet};
    pub use client::interact_entity::*;
    pub use client::title::SetTitle as _;
    pub use client::{
        despawn_disconnected_clients, Client, CompassPos, DeathLocation, HasRespawnScreen,
        HashedSeed, Ip, IsDebug, IsFlat, IsHardcore, OldView, OldViewDistance, PrevGameMode,
        Properties, ReducedDebugInfo, Username, View, ViewDistance,
    };
    pub use despawn::Despawned;
    pub use dimension::{DimensionType, DimensionTypeRegistry};
    pub use direction::Direction;
    pub use ecs::prelude::*;
    pub use entity::{
        EntityAnimation, EntityKind, EntityManager, EntityStatus, HeadYaw, Location, Look,
        OldLocation, OldPosition, Position,
    };
    pub use game_mode::GameMode;
    pub use glam::{DVec2, DVec3, Vec2, Vec3};
    pub use hand::Hand;
    pub use ident::Ident;
    pub use instance::{Block, BlockMut, BlockRef, Chunk, Instance};
    #[cfg(feature = "inventory")]
    pub use inventory::{
        CursorItem, Inventory, InventoryKind, InventoryWindow, InventoryWindowMut, OpenInventory,
    };
    pub use item::{ItemKind, ItemStack};
    pub use nbt::Compound;
    #[cfg(feature = "network")]
    pub use network::{
        ErasedNetworkCallbacks, NetworkCallbacks, NetworkSettings, NewClientInfo,
        SharedNetworkState,
    };
    pub use particle::Particle;
    #[cfg(feature = "player_list")]
    pub use player_list::{PlayerList, PlayerListEntry};
    pub use text::{Color, Text, TextFormat};
    #[cfg(feature = "advancement")]
    pub use valence_advancement::{
        event::AdvancementTabChange, Advancement, AdvancementBundle, AdvancementClientUpdate,
        AdvancementCriteria, AdvancementDisplay, AdvancementFrameType, AdvancementRequirements,
    };
    pub use valence_core::ident; // Export the `ident!` macro.
    pub use valence_core::uuid::UniqueId;
    pub use valence_core::{translation_key, CoreSettings, Server};
    pub use valence_entity::hitbox::{Hitbox, HitboxShape};

    pub use super::DefaultPlugins;
    use super::*;
}

/// This plugin group will add all the default plugins for a Valence
/// application.
///
/// [`DefaultPlugins`] obeys Cargo feature flags. Users may exert control over
/// this plugin group by disabling `default-features` in their `Cargo.toml` and
/// enabling only those features that they wish to use.
pub struct DefaultPlugins;

impl PluginGroup for DefaultPlugins {
    fn build(self) -> PluginGroupBuilder {
        #[allow(unused_mut)]
        let mut group = PluginGroupBuilder::start::<Self>()
            .add(valence_core::CorePlugin)
            .add(valence_registry::RegistryPlugin)
            .add(valence_biome::BiomePlugin)
            .add(valence_dimension::DimensionPlugin)
            .add(valence_entity::EntityPlugin)
            .add(valence_entity::hitbox::HitboxPlugin)
            .add(valence_instance::InstancePlugin)
            .add(valence_client::ClientPlugin);

        #[cfg(feature = "network")]
        {
            group = group.add(valence_network::NetworkPlugin);
        }

        #[cfg(feature = "player_list")]
        {
            group = group.add(valence_player_list::PlayerListPlugin);
        }

        #[cfg(feature = "inventory")]
        {
            group = group.add(valence_inventory::InventoryPlugin);
        }

        #[cfg(feature = "anvil")]
        {
            group = group.add(valence_anvil::AnvilPlugin);
        }

        #[cfg(feature = "advancement")]
        {
            group = group
                .add(valence_advancement::AdvancementPlugin)
                .add(valence_advancement::bevy_hierarchy::HierarchyPlugin);
        }

        #[cfg(feature = "world_border")]
        {
            group = group.add(valence_world_border::WorldBorderPlugin);
        }

        #[cfg(feature = "boss_bar")]
        {
            group = group.add(valence_boss_bar::BossBarPlugin);
        }

        group
    }
}
