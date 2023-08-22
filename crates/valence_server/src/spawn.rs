//! Handles spawning and respawning the client.

use std::borrow::Cow;
use std::collections::BTreeSet;

use bevy_ecs::prelude::*;
use bevy_ecs::query::WorldQuery;
use valence_entity::EntityLayerId;
use valence_protocol::packets::play::{GameJoinS2c, PlayerRespawnS2c, PlayerSpawnPositionS2c};
use valence_protocol::{BlockPos, GameMode, GlobalPos, Ident, VarInt, WritePacket};
use valence_registry::tags::TagsRegistry;
use valence_registry::{BiomeRegistry, RegistryCodec};

use crate::client::{Client, ViewDistance, VisibleChunkLayer};
use crate::layer::ChunkLayer;

// Components for the join game and respawn packet.

#[derive(Component, Clone, PartialEq, Eq, Default, Debug)]
pub struct DeathLocation(pub Option<(Ident<String>, BlockPos)>);

#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct IsHardcore(pub bool);

/// Hashed world seed used for biome noise.
#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct HashedSeed(pub u64);

#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct ReducedDebugInfo(pub bool);

#[derive(Component, Copy, Clone, PartialEq, Eq, Debug)]
pub struct HasRespawnScreen(pub bool);

/// If the client is spawning into a debug world.
#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct IsDebug(pub bool);

/// Changes the perceived horizon line (used for superflat worlds).
#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct IsFlat(pub bool);

#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct PortalCooldown(pub i32);

/// The initial previous gamemode. Used for the F3+F4 gamemode switcher.
#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct PrevGameMode(pub Option<GameMode>);

impl Default for HasRespawnScreen {
    fn default() -> Self {
        Self(true)
    }
}

/// The position and angle that clients will respawn with. Also
/// controls the position that compasses point towards.
#[derive(Component, Copy, Clone, PartialEq, Default, Debug)]
pub struct RespawnPosition {
    /// The position that clients will respawn at. This can be changed at any
    /// time to set the position that compasses point towards.
    pub pos: BlockPos,
    /// The yaw angle that clients will respawn with (in degrees).
    pub yaw: f32,
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

pub(super) fn initial_join(
    codec: Res<RegistryCodec>,
    tags: Res<TagsRegistry>,
    mut clients: Query<(&mut Client, &VisibleChunkLayer, ClientSpawnQueryReadOnly), Added<Client>>,
    chunk_layers: Query<&ChunkLayer>,
) {
    for (mut client, visible_chunk_layer, spawn) in &mut clients {
        let Ok(chunk_layer) = chunk_layers.get(visible_chunk_layer.0) else {
            continue;
        };

        let dimension_names: BTreeSet<Ident<Cow<str>>> = codec
            .registry(BiomeRegistry::KEY)
            .iter()
            .map(|value| value.name.as_str_ident().into())
            .collect();

        let dimension_name: Ident<Cow<str>> = chunk_layer.dimension_type_name().into();

        let last_death_location = spawn.death_loc.0.as_ref().map(|(id, pos)| GlobalPos {
            dimension_name: id.as_str_ident().into(),
            position: *pos,
        });

        // The login packet is prepended so that it's sent before all the other packets.
        // Some packets don't work correctly when sent before the game join packet.
        _ = client.enc.prepend_packet(&GameJoinS2c {
            entity_id: 0, // We reserve ID 0 for clients.
            is_hardcore: spawn.is_hardcore.0,
            game_mode: *spawn.game_mode,
            previous_game_mode: spawn.prev_game_mode.0.into(),
            dimension_names: Cow::Owned(dimension_names),
            registry_codec: Cow::Borrowed(codec.cached_codec()),
            dimension_type_name: dimension_name.clone(),
            dimension_name,
            hashed_seed: spawn.hashed_seed.0 as i64,
            max_players: VarInt(0), // Ignored by clients.
            view_distance: VarInt(spawn.view_distance.get() as i32),
            simulation_distance: VarInt(16), // TODO.
            reduced_debug_info: spawn.reduced_debug_info.0,
            enable_respawn_screen: spawn.has_respawn_screen.0,
            is_debug: spawn.is_debug.0,
            is_flat: spawn.is_flat.0,
            last_death_location,
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

pub(super) fn respawn(
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
    chunk_layers: Query<&ChunkLayer>,
) {
    for (mut client, loc, death_loc, hashed_seed, game_mode, prev_game_mode, is_debug, is_flat) in
        &mut clients
    {
        if client.is_added() {
            // No need to respawn since we are sending the game join packet this tick.
            continue;
        }

        let Ok(chunk_layer) = chunk_layers.get(loc.0) else {
            continue;
        };

        let dimension_name = chunk_layer.dimension_type_name();

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

/// Sets the client's respawn and compass position.
///
/// This also closes the "downloading terrain" screen when first joining, so
/// it should happen after the initial chunks are written.
pub(super) fn update_respawn_position(
    mut clients: Query<(&mut Client, &RespawnPosition), Changed<RespawnPosition>>,
) {
    for (mut client, respawn_pos) in &mut clients {
        client.write_packet(&PlayerSpawnPositionS2c {
            position: respawn_pos.pos,
            angle: respawn_pos.yaw,
        });
    }
}
