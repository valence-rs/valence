#![cfg_attr(
    unstable_doc,
    doc = "**❗ NOTE:** This documentation is sourced from the `main` branch. If you're looking for the most recent stable release, go [here](https://docs.rs/valence/latest/valence/).\n\n---\n"
)]
#![doc = include_str!("../README.md")]
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

#[cfg(feature = "testing")]
pub mod testing;

#[cfg(test)]
mod tests;

#[cfg(feature = "log")]
pub use bevy_log as log;
use registry::biome::BiomePlugin;
use registry::dimension_type::DimensionTypePlugin;
#[cfg(feature = "advancement")]
pub use valence_advancement as advancement;
#[cfg(feature = "anvil")]
pub use valence_anvil as anvil;
#[cfg(feature = "boss_bar")]
pub use valence_boss_bar as boss_bar;
#[cfg(feature = "command")]
pub use valence_command as command;
#[cfg(feature = "command")]
pub use valence_command_macros as command_macros;
#[cfg(feature = "inventory")]
pub use valence_inventory as inventory;
pub use valence_lang as lang;
#[cfg(feature = "network")]
pub use valence_network as network;
#[cfg(feature = "player_list")]
pub use valence_player_list as player_list;
use valence_registry::RegistryPlugin;
#[cfg(feature = "scoreboard")]
pub use valence_scoreboard as scoreboard;
use valence_server::abilities::AbilitiesPlugin;
use valence_server::action::ActionPlugin;
use valence_server::client::ClientPlugin;
use valence_server::client_command::ClientCommandPlugin;
use valence_server::client_settings::ClientSettingsPlugin;
use valence_server::custom_payload::CustomPayloadPlugin;
use valence_server::entity::hitbox::HitboxPlugin;
use valence_server::entity::EntityPlugin;
use valence_server::event_loop::EventLoopPlugin;
use valence_server::hand_swing::HandSwingPlugin;
use valence_server::interact_block::InteractBlockPlugin;
use valence_server::interact_entity::InteractEntityPlugin;
use valence_server::interact_item::InteractItemPlugin;
use valence_server::keepalive::KeepalivePlugin;
use valence_server::layer::LayerPlugin;
use valence_server::message::MessagePlugin;
use valence_server::movement::MovementPlugin;
use valence_server::op_level::OpLevelPlugin;
pub use valence_server::protocol::status_effects;
use valence_server::resource_pack::ResourcePackPlugin;
use valence_server::status::StatusPlugin;
use valence_server::teleport::TeleportPlugin;
pub use valence_server::*;
#[cfg(feature = "weather")]
pub use valence_weather as weather;
#[cfg(feature = "world_border")]
pub use valence_world_border as world_border;

/// Contains the most frequently used items in Valence projects.
///
/// This is usually glob imported like so:
///
/// ```
/// use valence::prelude::*; // Glob import.
///
/// let mut app = App::new();
/// app.add_systems(Update, || println!("yippee!"));
/// // ...
/// ```
pub mod prelude {
    pub use bevy_app::prelude::*;
    pub use bevy_ecs; // Needed for bevy_ecs macros to function correctly.
    pub use bevy_ecs::prelude::*;
    pub use uuid::Uuid;
    #[cfg(feature = "advancement")]
    pub use valence_advancement::{
        event::AdvancementTabChangeEvent, Advancement, AdvancementBundle, AdvancementClientUpdate,
        AdvancementCriteria, AdvancementDisplay, AdvancementFrameType, AdvancementRequirements,
    };
    #[cfg(feature = "inventory")]
    pub use valence_inventory::{
        CursorItem, Inventory, InventoryKind, InventoryWindow, InventoryWindowMut, OpenInventory,
    };
    #[cfg(feature = "network")]
    pub use valence_network::{
        ConnectionMode, ErasedNetworkCallbacks, NetworkCallbacks, NetworkSettings, NewClientInfo,
        SharedNetworkState,
    };
    #[cfg(feature = "player_list")]
    pub use valence_player_list::{PlayerList, PlayerListEntry};
    pub use valence_registry::biome::{Biome, BiomeId, BiomeRegistry};
    pub use valence_registry::dimension_type::{DimensionType, DimensionTypeRegistry};
    pub use valence_server::action::{DiggingEvent, DiggingState};
    pub use valence_server::block::{BlockKind, BlockState, PropName, PropValue};
    pub use valence_server::client::{
        despawn_disconnected_clients, Client, Ip, OldView, OldViewDistance, Properties, Username,
        View, ViewDistance, VisibleChunkLayer, VisibleEntityLayers,
    };
    pub use valence_server::client_command::{
        ClientCommand, JumpWithHorseEvent, JumpWithHorseState, LeaveBedEvent, SneakEvent,
        SneakState, SprintEvent, SprintState,
    };
    pub use valence_server::entity::hitbox::{Hitbox, HitboxShape};
    pub use valence_server::entity::{
        EntityAnimation, EntityKind, EntityLayerId, EntityManager, EntityStatus, HeadYaw, Look,
        OldEntityLayerId, OldPosition, Position,
    };
    pub use valence_server::event_loop::{
        EventLoopPostUpdate, EventLoopPreUpdate, EventLoopUpdate,
    };
    pub use valence_server::ident::Ident;
    pub use valence_server::interact_entity::{EntityInteraction, InteractEntityEvent};
    pub use valence_server::layer::chunk::{
        Block, BlockRef, Chunk, ChunkLayer, LoadedChunk, UnloadedChunk,
    };
    pub use valence_server::layer::{EntityLayer, LayerBundle};
    pub use valence_server::math::{DVec2, DVec3, Vec2, Vec3};
    pub use valence_server::message::SendMessage as _;
    pub use valence_server::nbt::Compound;
    pub use valence_server::protocol::packets::play::particle_s2c::Particle;
    pub use valence_server::protocol::text::{Color, IntoText, Text};
    pub use valence_server::spawn::{ClientSpawnQuery, ClientSpawnQueryReadOnly, RespawnPosition};
    pub use valence_server::title::SetTitle as _;
    pub use valence_server::{
        ident, BlockPos, ChunkPos, ChunkView, Despawned, Direction, GameMode, Hand, ItemKind,
        ItemStack, Server, UniqueId,
    };

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
            .add(ServerPlugin)
            .add(RegistryPlugin)
            .add(BiomePlugin)
            .add(DimensionTypePlugin)
            .add(EntityPlugin)
            .add(HitboxPlugin)
            .add(LayerPlugin)
            .add(ClientPlugin)
            .add(EventLoopPlugin)
            .add(MovementPlugin)
            .add(ClientCommandPlugin)
            .add(KeepalivePlugin)
            .add(InteractEntityPlugin)
            .add(ClientSettingsPlugin)
            .add(ActionPlugin)
            .add(TeleportPlugin)
            .add(MessagePlugin)
            .add(CustomPayloadPlugin)
            .add(HandSwingPlugin)
            .add(InteractBlockPlugin)
            .add(InteractItemPlugin)
            .add(OpLevelPlugin)
            .add(ResourcePackPlugin)
            .add(StatusPlugin)
            .add(AbilitiesPlugin);

        #[cfg(feature = "log")]
        {
            group = group.add(bevy_log::LogPlugin::default());
        }

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
            group = group.add(valence_advancement::AdvancementPlugin)
        }

        #[cfg(feature = "weather")]
        {
            group = group.add(valence_weather::WeatherPlugin);
        }

        #[cfg(feature = "world_border")]
        {
            group = group.add(valence_world_border::WorldBorderPlugin);
        }

        #[cfg(feature = "boss_bar")]
        {
            group = group.add(valence_boss_bar::BossBarPlugin);
        }

        #[cfg(feature = "command")]
        {
            group = group.add(valence_command::manager::CommandPlugin);
        }

        #[cfg(feature = "scoreboard")]
        {
            group = group.add(valence_scoreboard::ScoreboardPlugin);
        }

        group
    }
}
