//! Handles spawning and respawning the client.

use std::borrow::Cow;
use std::collections::BTreeSet;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::query::WorldQuery;
use derive_more::{Deref, DerefMut};
use valence_entity::EntityLayerId;
use valence_protocol::packets::play::{GameJoinS2c, PlayerRespawnS2c};
use valence_protocol::{BlockPos, GameMode, GlobalPos, Ident, VarInt, WritePacket};
use valence_registry::tags::TagsRegistry;
use valence_registry::{DimensionTypeRegistry, RegistryCodec, UpdateRegistrySet};

use crate::client::{Client, FlushPacketsSet, ViewDistance, VisibleChunkLayer};
use crate::dimension_layer::{ChunkIndex, DimensionInfo, UpdateDimensionLayerSet};

/// Handles spawning and respawning of clients.
pub struct SpawnPlugin;

/// When clients are sent the "respawn" packet after their dimension layer has
/// changed.
#[derive(SystemSet, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct RespawnSystemSet;

impl Plugin for SpawnPlugin {
    fn build(&self, app: &mut App) {
        app
            // Send the respawn packet before chunks are sent.
            .configure_set(PostUpdate, RespawnSystemSet.before(UpdateDimensionLayerSet))
            .add_systems(PostUpdate, respawn.in_set(RespawnSystemSet))
            // The join game packet is prepended to the client's packet buffer, so
            // it can be sent any time before packets are flushed. Additionally,
            // this must be scheduled after registries are updated because we read
            // the cached packets.
            .add_systems(PostUpdate, initial_join.after(UpdateRegistrySet).before(FlushPacketsSet));
    }
}

/// A convenient [`WorldQuery`] for obtaining client spawn components. Also see
/// [`ClientSpawnQueryReadOnly`].
#[derive(WorldQuery)]
#[world_query(mutable)]
pub struct ClientSpawnQuery {
    pub is_hardcore: &'static mut IsHardcore,
    pub game_mode: &'static mut GameMode,
    pub prev_game_mode: &'static mut PrevGameMode,
    pub hashed_seed: &'static mut HashedSeed,
    pub view_distance: &'static mut ViewDistance,
    pub reduced_debug_info: &'static mut ReducedDebugInfo,
    pub has_respawn_screen: &'static mut HasRespawnScreen,
    pub is_debug: &'static mut IsDebug,
    pub is_flat: &'static mut IsFlat,
    pub death_loc: &'static mut DeathLocation,
    pub portal_cooldown: &'static mut PortalCooldown,
}

// Components for the join game and respawn packet.

#[derive(Component, Clone, PartialEq, Eq, Default, Debug, Deref, DerefMut)]
pub struct DeathLocation(pub Option<(Ident<String>, BlockPos)>);

impl DeathLocation {
    pub fn as_global_pos(&self) -> Option<GlobalPos> {
        self.0.as_ref().map(|(name, pos)| GlobalPos {
            dimension_name: name.as_str_ident().into(),
            position: *pos,
        })
    }
}

#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug, Deref, DerefMut)]
pub struct IsHardcore(pub bool);

/// Hashed world seed used for biome noise.
#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug, Deref, DerefMut)]
pub struct HashedSeed(pub u64);

#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug, Deref, DerefMut)]
pub struct ReducedDebugInfo(pub bool);

#[derive(Component, Copy, Clone, PartialEq, Eq, Debug, Deref, DerefMut)]
pub struct HasRespawnScreen(pub bool);

impl Default for HasRespawnScreen {
    fn default() -> Self {
        Self(true)
    }
}

/// If the client is spawning into a debug world.
#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug, Deref, DerefMut)]
pub struct IsDebug(pub bool);

/// Changes the perceived horizon line (used for superflat worlds).
#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug, Deref, DerefMut)]
pub struct IsFlat(pub bool);

#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug, Deref, DerefMut)]
pub struct PortalCooldown(pub i32);

/// The initial previous gamemode. Used for the F3+F4 gamemode switcher.
#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug, Deref, DerefMut)]
pub struct PrevGameMode(pub Option<GameMode>);

pub(super) fn initial_join(
    mut clients: Query<(&mut Client, &VisibleChunkLayer, ClientSpawnQueryReadOnly), Added<Client>>,
    codec: Res<RegistryCodec>,
    tags: Res<TagsRegistry>,
    dimensions: Res<DimensionTypeRegistry>,
    dimension_layers: Query<(&ChunkIndex, &DimensionInfo)>,
) {
    for (mut client, visible_chunk_layer, spawn) in &mut clients {
        let Ok((chunk_index, info)) = dimension_layers.get(visible_chunk_layer.0) else {
            continue;
        };

        let dimension_names: BTreeSet<Ident<Cow<str>>> = codec
            .registry(DimensionTypeRegistry::KEY)
            .iter()
            .map(|value| value.name.as_str_ident().into())
            .collect();

        let dimension_name = dimensions.by_index(info.dimension_type()).0;

        // The login packet is prepended so that it's sent before all the other packets.
        // Some packets don't work correctly when sent before the game join packet.
        _ = client.enc.prepend_packet(&GameJoinS2c {
            entity_id: 0, // We reserve ID 0 for clients.
            is_hardcore: spawn.is_hardcore.0,
            game_mode: *spawn.game_mode,
            previous_game_mode: spawn.prev_game_mode.0.into(),
            dimension_names: Cow::Owned(dimension_names),
            registry_codec: Cow::Borrowed(codec.cached_codec()),
            dimension_type_name: dimension_name.into(),
            dimension_name: dimension_name.into(),
            hashed_seed: spawn.hashed_seed.0 as i64,
            max_players: VarInt(0), // Ignored by clients.
            view_distance: VarInt(spawn.view_distance.get() as i32),
            simulation_distance: VarInt(16), // Ignored?
            reduced_debug_info: spawn.reduced_debug_info.0,
            enable_respawn_screen: spawn.has_respawn_screen.0,
            is_debug: spawn.is_debug.0,
            is_flat: spawn.is_flat.0,
            last_death_location: spawn.death_loc.as_global_pos(),
            portal_cooldown: VarInt(spawn.portal_cooldown.0),
        });

        client.write_packet_bytes(tags.sync_tags_packet());

        /*
        // TODO: enable all the features?
        q.client.write_packet(&FeatureFlags {
            features: vec![Ident::new("vanilla").unwrap()],
        })?;
        */
    }
}

fn respawn(
    mut clients: Query<
        (
            &mut Client,
            &EntityLayerId,
            &DeathLocation,
            &HashedSeed,
            &GameMode,
            &PrevGameMode,
            &IsDebug,
            &IsFlat,
        ),
        Changed<VisibleChunkLayer>,
    >,
    chunk_layers: Query<(&ChunkIndex, &DimensionInfo)>,
    dimensions: Res<DimensionTypeRegistry>,
) {
    for (mut client, loc, death_loc, hashed_seed, game_mode, prev_game_mode, is_debug, is_flat) in
        &mut clients
    {
        if client.is_added() {
            // No need to respawn since we are sending the game join packet this tick.
            continue;
        }

        let Ok((chunk_index, info)) = chunk_layers.get(loc.0) else {
            continue;
        };

        let dimension_name = dimensions.by_index(info.dimension_type()).0;

        let last_death_location = death_loc.0.as_ref().map(|(id, pos)| GlobalPos {
            dimension_name: id.as_str_ident().into(),
            position: *pos,
        });

        client.write_packet(&PlayerRespawnS2c {
            dimension_type_name: dimension_name.into(),
            dimension_name: dimension_name.into(),
            hashed_seed: hashed_seed.0,
            game_mode: *game_mode,
            previous_game_mode: prev_game_mode.0.into(),
            is_debug: is_debug.0,
            is_flat: is_flat.0,
            copy_metadata: true,
            last_death_location,
            portal_cooldown: VarInt(0), // TODO
        });
    }
}
