#![doc = include_str!("../README.md")] // Points to the main project README.
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
#[cfg(feature = "boss_bar")]
pub use valence_boss_bar as boss_bar;
pub use valence_core::*;
#[cfg(feature = "inventory")]
pub use valence_inventory as inventory;
#[cfg(feature = "network")]
pub use valence_network as network;
#[cfg(feature = "player_list")]
pub use valence_player_list as player_list;
#[cfg(feature = "world_border")]
pub use valence_world_border as world_border;
pub use {
    bevy_app, bevy_ecs, glam, valence_biome as biome, valence_block as block,
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
    pub use bevy_app::prelude::*;
    pub use bevy_ecs; // Needed for bevy_ecs macros to function correctly.
    pub use bevy_ecs::prelude::*;
    pub use glam::{DVec2, DVec3, Vec2, Vec3};
    pub use ident::Ident;
    pub use uuid::Uuid;
    #[cfg(feature = "advancement")]
    pub use valence_advancement::{
        event::AdvancementTabChangeEvent, Advancement, AdvancementBundle, AdvancementClientUpdate,
        AdvancementCriteria, AdvancementDisplay, AdvancementFrameType, AdvancementRequirements,
    };
    pub use valence_biome::{Biome, BiomeId, BiomeRegistry};
    pub use valence_block::{BlockKind, BlockState, PropName, PropValue};
    pub use valence_client::action::{DiggingEvent, DiggingState};
    pub use valence_client::command::{
        ClientCommand, JumpWithHorseEvent, JumpWithHorseState, LeaveBedEvent, SneakEvent,
        SneakState, SprintEvent, SprintState,
    };
    pub use valence_client::event_loop::{
        EventLoopPostUpdate, EventLoopPreUpdate, EventLoopUpdate,
    };
    pub use valence_client::interact_entity::{EntityInteraction, InteractEntityEvent};
    pub use valence_client::title::SetTitle as _;
    pub use valence_client::{
        despawn_disconnected_clients, Client, DeathLocation, HasRespawnScreen, HashedSeed, Ip,
        IsDebug, IsFlat, IsHardcore, OldView, OldViewDistance, PrevGameMode, Properties,
        ReducedDebugInfo, RespawnPosition, Username, View, ViewDistance,
    };
    pub use valence_core::block_pos::BlockPos;
    pub use valence_core::chunk_pos::{ChunkPos, ChunkView};
    pub use valence_core::despawn::Despawned;
    pub use valence_core::direction::Direction;
    pub use valence_core::game_mode::GameMode;
    pub use valence_core::hand::Hand;
    pub use valence_core::ident; // Export the `ident!` macro.
    pub use valence_core::item::{ItemKind, ItemStack};
    pub use valence_core::particle::Particle;
    pub use valence_core::text::{Color, Text, TextFormat};
    pub use valence_core::uuid::UniqueId;
    pub use valence_core::{translation_key, CoreSettings, Server};
    pub use valence_dimension::{DimensionType, DimensionTypeRegistry};
    pub use valence_entity::hitbox::{Hitbox, HitboxShape};
    pub use valence_entity::{
        EntityAnimation, EntityKind, EntityManager, EntityStatus, HeadYaw, Location, Look,
        OldLocation, OldPosition, Position,
    };
    pub use valence_instance::{Block, BlockMut, BlockRef, Chunk, Instance};
    #[cfg(feature = "inventory")]
    pub use valence_inventory::{
        CursorItem, Inventory, InventoryKind, InventoryWindow, InventoryWindowMut, OpenInventory,
    };
    pub use valence_nbt::Compound;
    #[cfg(feature = "network")]
    pub use valence_network::{
        ConnectionMode, ErasedNetworkCallbacks, NetworkCallbacks, NetworkSettings, NewClientInfo,
        SharedNetworkState,
    };
    #[cfg(feature = "player_list")]
    pub use valence_player_list::{PlayerList, PlayerListEntry};

    pub use super::DefaultPlugins;
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
