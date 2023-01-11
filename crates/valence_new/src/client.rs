use std::collections::HashSet;
use std::fmt;
use std::net::IpAddr;

use anyhow::{bail, Context};
use bevy_ecs::prelude::*;
use glam::DVec3;
use tokio::sync::OwnedSemaphorePermit;
use tracing::warn;
use uuid::Uuid;
use valence_protocol::packets::s2c::play::{
    DisconnectPlay, GameEvent, KeepAliveS2c, LoginPlayOwned, RespawnOwned, SetCenterChunk,
    SetDefaultSpawnPosition, SetRenderDistance, SynchronizePlayerPosition, UnloadChunk,
};
use valence_protocol::types::{GameEventKind, GameMode, SyncPlayerPosLookFlags};
use valence_protocol::{BlockPos, EncodePacket, Username, VarInt};

use crate::chunk_pos::ChunkPos;
use crate::dimension::DimensionId;
use crate::entity::McEntity;
use crate::instance::Instance;
use crate::server::{NewClientInfo, PlayPacketReceiver, PlayPacketSender, Server};
use crate::NULL_ENTITY;

pub mod event;

#[derive(Component)]
pub struct Client {
    send: PlayPacketSender,
    recv: PlayPacketReceiver,
    is_disconnected: bool,
    /// Ensures that we don't allow more connections to the server until the
    /// client is dropped.
    _permit: OwnedSemaphorePermit,
    /// To make sure we're not loading already loaded chunks, or unloading
    /// unloaded chunks.
    #[cfg(debug_assertions)]
    loaded_chunks: HashSet<ChunkPos>,
    username: Username<String>,
    uuid: Uuid,
    ip: IpAddr,
    instance: Entity,
    old_instance: Entity,
    new_instance: Entity,
    position: DVec3,
    old_position: DVec3,
    yaw: f32,
    pitch: f32,
    game_mode: GameMode,
    block_change_sequence: i32,
    view_distance: u8,
    old_view_distance: u8,
    death_location: Option<(DimensionId, BlockPos)>,
    entities_to_despawn: Vec<VarInt>,
    got_keepalive: bool,
    last_keepalive_id: u64,
    /// Counts up as teleports are made.
    teleport_id_counter: u32,
    /// The number of pending client teleports that have yet to receive a
    /// confirmation. Inbound client position packets should be ignored while
    /// this is nonzero.
    pending_teleports: u32,
    /// If the client needs initialization.
    is_new: bool,
    /// If the client needs to be sent the respawn packet for the current world.
    needs_respawn: bool,
    is_hardcore: bool,
    is_flat: bool,
    has_respawn_screen: bool,
}

impl Client {
    pub(crate) fn new(
        send: PlayPacketSender,
        recv: PlayPacketReceiver,
        permit: OwnedSemaphorePermit,
        info: NewClientInfo,
    ) -> Self {
        Self {
            send,
            recv,
            is_disconnected: false,
            _permit: permit,
            #[cfg(debug_assertions)]
            loaded_chunks: HashSet::new(),
            username: info.username,
            uuid: info.uuid,
            ip: info.ip,
            instance: NULL_ENTITY,
            old_instance: NULL_ENTITY,
            new_instance: NULL_ENTITY,
            position: DVec3::ZERO,
            old_position: DVec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            game_mode: GameMode::default(),
            block_change_sequence: 0,
            view_distance: 2,
            old_view_distance: 2,
            death_location: None,
            entities_to_despawn: vec![],
            is_new: true,
            needs_respawn: false,
            is_hardcore: false,
            is_flat: false,
            has_respawn_screen: false,
            got_keepalive: true,
            last_keepalive_id: 0,
            teleport_id_counter: 0,
            pending_teleports: 0,
        }
    }

    /// Attempts to write a play packet into this client's packet buffer. The
    /// packet will be sent at the end of the tick.
    ///
    /// If encoding the packet fails, the client is disconnected. Has no
    /// effect if the client is already disconnected.
    pub fn write_packet<P>(&mut self, pkt: &P)
    where
        P: EncodePacket + fmt::Debug + ?Sized,
    {
        if let Err(e) = self.send.append_packet(pkt) {
            if !self.is_disconnected {
                self.is_disconnected = true;
                warn!(
                    username = %self.username,
                    uuid = %self.uuid,
                    ip = %self.ip,
                    "failed to write packet: {e:#}"
                );
            }
        }
    }

    /// Writes arbitrary bytes to this client's packet buffer. Don't use this
    /// function unless you know what you're doing. Consider using
    /// [`write_packet`] instead.
    ///
    /// [`write_packet`]: Self::write_packet
    pub fn write_packet_bytes(&mut self, bytes: &[u8]) {
        self.send.append_bytes(bytes);
    }

    pub(crate) fn despawn_entity(&mut self, protocol_id: i32) {
        todo!("push protocol id to buffer");
    }

    /// Gets the username of this client.
    pub fn username(&self) -> Username<&str> {
        self.username.as_str_username()
    }

    /// Gets the UUID of this client.
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    /// Gets the IP address of this client.
    pub fn ip(&self) -> IpAddr {
        self.ip
    }

    /// Gets whether or not the client is connected to the server.
    ///
    /// A disconnected client component will never become reconnected. It is
    /// your responsibility to despawn disconnected client entities, since
    /// they will not be automatically despawned by Valence.
    pub fn is_disconnected(&self) -> bool {
        self.is_disconnected
    }

    /// Gets the [`Instance`] entity this client is located in. The client is
    /// not in any instance when they first join.
    pub fn instance(&self) -> Entity {
        self.instance
    }

    /// Sets the [`Instance`] entity this client is located in. This can be used
    /// to respawn the client after death.
    ///
    /// The given [`Entity`] must exist and have the [`Instance`] component.
    /// Otherwise, the client is disconnected at the end of the tick.
    pub fn set_instance(&mut self, instance: Entity) {
        self.instance = instance;
        self.needs_respawn = true;
    }

    /// Gets the absolute position of this client in the instance it is located
    /// in.
    pub fn position(&self) -> DVec3 {
        self.position
    }

    /// Gets the position of this client at the end of the previous tick.
    pub fn old_position(&self) -> DVec3 {
        self.old_position
    }

    /// Gets this client's yaw (in degrees).
    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    /// Gets this client's pitch (in degrees).
    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// Changes the position and rotation of this client in the world it is
    /// located in.
    ///
    /// If you want to change the client's world, use [`Self::respawn`].
    pub fn teleport(&mut self, pos: impl Into<DVec3>, yaw: f32, pitch: f32) {
        self.position = pos.into();
        self.yaw = yaw;
        self.pitch = pitch;

        self.write_packet(&SynchronizePlayerPosition {
            position: self.position.to_array(),
            yaw,
            pitch,
            flags: SyncPlayerPosLookFlags::new(),
            teleport_id: VarInt(self.teleport_id_counter as i32),
            dismount_vehicle: false,
        });

        self.pending_teleports = self.pending_teleports.wrapping_add(1);
        self.teleport_id_counter = self.teleport_id_counter.wrapping_add(1);
    }

    pub fn has_respawn_screen(&self) -> bool {
        self.has_respawn_screen
    }

    /// Sets whether respawn screen should be displayed after client's death.
    pub fn set_respawn_screen(&mut self, enable: bool) {
        if self.has_respawn_screen != enable {
            self.has_respawn_screen = enable;

            if !self.is_new {
                self.write_packet(&GameEvent {
                    kind: GameEventKind::EnableRespawnScreen,
                    value: if enable { 0.0 } else { 1.0 },
                });
            }
        }
    }

    /// The current view distance of this client measured in chunks. The client
    /// will not be able to see chunks and entities past this distance.
    ///
    /// The result is in `2..=32`.
    pub fn view_distance(&self) -> u8 {
        self.view_distance
    }

    pub(crate) fn old_view_distance(&self) -> u8 {
        self.old_view_distance
    }

    /// Sets the view distance. The client will not be able to see chunks and
    /// entities past this distance.
    ///
    /// The new view distance is measured in chunks and is clamped to `2..=32`.
    pub fn set_view_distance(&mut self, dist: u8) {
        self.view_distance = dist.clamp(2, 32);
    }

    /// Gets the last death location of this client. The client will see
    /// `minecraft:recovery_compass` items point at the returned position.
    ///
    /// If the client's current dimension differs from the returned
    /// dimension or the location is `None` then the compass will spin
    /// randomly.
    pub fn death_location(&self) -> Option<(DimensionId, BlockPos)> {
        self.death_location
    }

    /// Gets the client's game mode.
    pub fn game_mode(&self) -> GameMode {
        self.game_mode
    }

    /// Sets the client's game mode.
    pub fn set_game_mode(&mut self, game_mode: GameMode) {
        if self.game_mode != game_mode {
            self.game_mode = game_mode;

            if !self.is_new {
                self.write_packet(&GameEvent {
                    kind: GameEventKind::ChangeGameMode,
                    value: game_mode as i32 as f32,
                });
            }
        }
    }

    /// Sets the last death location. The client will see
    /// `minecraft:recovery_compass` items point at the provided position.
    /// If the client's current dimension differs from the provided
    /// dimension or the location is `None` then the compass will spin
    /// randomly.
    ///
    /// Changes to the last death location take effect when the client
    /// (re)spawns.
    pub fn set_death_location(&mut self, location: Option<(DimensionId, BlockPos)>) {
        self.death_location = location;
    }
}

pub(crate) fn update_clients(
    server: Res<Server>,
    mut clients: Query<(&mut Client, Option<&McEntity>)>,
    instances: Query<&mut Instance>,
    entities: Query<&McEntity>,
) {
    for (mut client, self_entity) in &mut clients {
        if !client.is_disconnected() {
            if let Err(e) =
                update_one_client(&mut client, self_entity, &server, &instances, &entities)
            {
                let _ = client.write_packet(&DisconnectPlay { reason: "".into() });
                client.is_disconnected = true;
                warn!(
                    username = %client.username,
                    uuid = %client.uuid,
                    ip = %client.ip,
                    "error updating client: {e:#}"
                );
            }
        }

        client.is_new = false;
    }
}

fn update_one_client(
    client: &mut Client,
    self_entity: Option<&McEntity>,
    server: &Server,
    instances: &Query<&mut Instance>,
    entities: &Query<&McEntity>,
) -> anyhow::Result<()> {
    let Ok(instance) = instances.get(client.instance) else {
        bail!("the client is not in an instance")
    };

    // Send the login (play) packet and other initial packets. We defer this until
    // now so that the user can set the client's initial location, game
    // mode, etc.
    if client.is_new {
        client.needs_respawn = false;

        let dimension_names: Vec<_> = server
            .dimensions()
            .map(|(id, _)| id.dimension_name())
            .collect();

        // The login packet is prepended so that it is sent before all the other
        // packets. Some packets don't work correctly when sent before the login packet,
        // which is why we're doing this.
        client.send.prepend_packet(&LoginPlayOwned {
            entity_id: 0, // ID 0 is reserved for clients.
            is_hardcore: client.is_hardcore,
            game_mode: client.game_mode,
            previous_game_mode: -1,
            dimension_names,
            registry_codec: server.registry_codec().clone(),
            dimension_type_name: instance.dimension().dimension_type_name(),
            dimension_name: instance.dimension().dimension_name(),
            hashed_seed: 42,
            max_players: VarInt(0), // Unused
            view_distance: VarInt(client.view_distance() as i32),
            simulation_distance: VarInt(16),
            reduced_debug_info: false,
            enable_respawn_screen: client.has_respawn_screen,
            is_debug: false,
            is_flat: client.is_flat,
            last_death_location: client
                .death_location
                .map(|(id, pos)| (id.dimension_name(), pos)),
        })?;

        /*
        // TODO: enable all the features?
        send.append_packet(&FeatureFlags {
            features: vec![Ident::new("vanilla").unwrap()],
        })?;
        */

        // TODO: write player list init packets.
    } else {
        if client.view_distance != client.old_view_distance {
            // Change the render distance fog.
            client
                .send
                .append_packet(&SetRenderDistance(VarInt(client.view_distance.into())))?;
        }

        if client.needs_respawn {
            client.needs_respawn = false;

            client.send.append_packet(&RespawnOwned {
                dimension_type_name: instance.dimension().dimension_type_name(),
                dimension_name: instance.dimension().dimension_name(),
                hashed_seed: 0,
                game_mode: client.game_mode,
                previous_game_mode: -1,
                is_debug: false,
                is_flat: client.is_flat,
                copy_metadata: true,
                last_death_location: client
                    .death_location
                    .map(|(id, pos)| (id.dimension_name(), pos)),
            })?;
        }

        // TODO: update changed player list.
    }

    // Check if it's time to send another keepalive.
    if server.current_tick() % (server.tick_rate() * 10) == 0 {
        if client.got_keepalive {
            let id = rand::random();
            client.send.append_packet(&KeepAliveS2c { id })?;
            client.last_keepalive_id = id;
            client.got_keepalive = false;
        } else {
            bail!("timed out (no keepalive response)");
        }
    }

    /*
    let self_entity_pos;
    let self_entity_instance;
    let self_entity_range;

    // TODO: attempt to check if the client's self-entity was changed and respond
    //       accordingly.

    if let Some(entity) = self_entity {
        self_entity_pos = ChunkPos::at(entity.position().x, entity.position().z);
        self_entity_instance = entity.instance();
        self_entity_range = entity.self_update_range.clone();
    } else {
        // Client isn't associated with a McEntity.
        self_entity_pos = ChunkPos::default();
        self_entity_instance = NULL_ENTITY;
        self_entity_range = 0..0;
    }
    */

    // The client's own chunk pos.
    let old_chunk_pos = ChunkPos::at(client.old_position.x, client.old_position.z);
    let chunk_pos = ChunkPos::at(client.position.x, client.position.z);

    // Make sure the center chunk is set /before/ loading chunks.
    if old_chunk_pos != chunk_pos {
        client.send.append_packet(&SetCenterChunk {
            chunk_x: VarInt(chunk_pos.x),
            chunk_z: VarInt(chunk_pos.z),
        })?;
    }

    if client.old_instance != client.new_instance {
        // Unload all chunks and entities in old view.
        if let Ok(old_instance) = instances.get(client.old_instance) {
            // TODO: only send unload packets when old dimension == new
            // dimension, since the       client will do the
            // unloading for us in that case?

            // old_chunk_pos.try_for_each_in_view(self.old_view_distance, |pos|
            // {     if let Some(cell) = old_instance.cell(pos) {
            //         if let Some(chunk) = &cell.chunk {
            //
            //         }
            //     }
            //
            //     Ok(())
            // })?;
        }
    }

    // TODO: load chunks here.

    if client.is_new {
        // This closes the "downloading terrain" screen.
        // Send this after the initial chunks are loaded.
        client.send.append_packet(&SetDefaultSpawnPosition {
            position: BlockPos::at(client.position),
            angle: client.yaw,
        })?;
    }

    client.old_instance = client.instance;
    client.old_position = client.position;
    client.old_view_distance = client.view_distance;

    client
        .send
        .flush()
        .context("failed to flush packet queue")?;

    Ok(())
}

/*
/// The system for updating clients.
pub(crate) fn update_clients(
    mut clients: Query<(&mut Client, Option<&McEntity>)>,
    server: Res<Server>,
    instances: Query<&Instance>,
    entities: Query<&McEntity>,
) {
    // TODO: what batch size to use?
    clients.par_for_each_mut(1, |(mut client, self_entity)| {
        if let Some(mut send) = client.send.take() {
            match update_one_client(
                &mut client,
                self_entity,
                &mut send,
                &server,
                &instances,
                &entities,
            ) {
                Ok(()) => client.send = Some(send),
                Err(e) => {
                    let _ = send.append_packet(&DisconnectPlay { reason: "".into() });
                    warn!(
                        username = %client.username,
                        uuid = %client.uuid,
                        ip = %client.ip,
                        "error updating client: {e:#}"
                    );
                }
            }
        }

        client.is_new = false;
    });
}

fn update_one_client(
    client: &mut Client,
    self_entity: Option<&McEntity>,
    send: &mut PlayPacketSender,
    server: &Server,
    instances: &Query<&Instance>,
    entities: &Query<&McEntity>,
) -> anyhow::Result<()> {
    let Ok(instance) = instances.get(client.instance) else {
        bail!("the client is not in an instance")
    };

    // Send the login (play) packet and other initial packets. We defer this until
    // now so that the user can set the client's initial location, game
    // mode, etc.
    if client.is_new {
        client.needs_respawn = false;

        let dimension_names: Vec<_> = server
            .dimensions()
            .map(|(id, _)| id.dimension_name())
            .collect();

        // The login packet is prepended so that it is sent before all the other
        // packets. Some packets don't work correctly when sent before the login packet,
        // which is why we're doing this.
        send.prepend_packet(&LoginPlayOwned {
            entity_id: 0, // ID 0 is reserved for clients.
            is_hardcore: client.is_hardcore,
            game_mode: client.game_mode,
            previous_game_mode: -1,
            dimension_names,
            registry_codec: server.registry_codec().clone(),
            dimension_type_name: instance.dimension().dimension_type_name(),
            dimension_name: instance.dimension().dimension_name(),
            hashed_seed: 42,
            max_players: VarInt(0), // Unused
            view_distance: VarInt(client.view_distance() as i32),
            simulation_distance: VarInt(16),
            reduced_debug_info: false,
            enable_respawn_screen: client.has_respawn_screen,
            is_debug: false,
            is_flat: client.is_flat,
            last_death_location: client
                .death_location
                .map(|(id, pos)| (id.dimension_name(), pos)),
        })?;

        /*
        // TODO: enable all the features?
        send.append_packet(&FeatureFlags {
            features: vec![Ident::new("vanilla").unwrap()],
        })?;
        */

        // TODO: write player list init packets.
    } else {
        if client.view_distance != client.old_view_distance {
            // Change the render distance fog.
            send.append_packet(&SetRenderDistance(VarInt(client.view_distance.into())))?;
        }

        if client.needs_respawn {
            client.needs_respawn = false;

            send.append_packet(&RespawnOwned {
                dimension_type_name: instance.dimension().dimension_type_name(),
                dimension_name: instance.dimension().dimension_name(),
                hashed_seed: 0,
                game_mode: client.game_mode,
                previous_game_mode: -1,
                is_debug: false,
                is_flat: client.is_flat,
                copy_metadata: true,
                last_death_location: client
                    .death_location
                    .map(|(id, pos)| (id.dimension_name(), pos)),
            })?;
        }

        // TODO: update changed player list.
    }

    // Check if it's time to send another keepalive.
    if server.current_tick() % (server.tick_rate() * 10) == 0 {
        if client.got_keepalive {
            let id = rand::random();
            send.append_packet(&KeepAliveS2c { id })?;
            client.last_keepalive_id = id;
            client.got_keepalive = false;
        } else {
            bail!("timed out (no keepalive response)");
        }
    }

    let self_entity_pos;
    let self_entity_instance;
    let self_entity_range;

    // TODO: attempt to check if the client's self-entity was changed and respond
    //       accordingly.

    if let Some(entity) = self_entity {
        self_entity_pos = ChunkPos::at(entity.position().x, entity.position().z);
        self_entity_instance = entity.instance();
        self_entity_range = entity.self_update_range.clone();
    } else {
        // Client isn't associated with a McEntity.
        self_entity_pos = ChunkPos::default();
        self_entity_instance = NULL_ENTITY;
        self_entity_range = 0..0;
    }

    // The client's own chunk pos.
    let old_chunk_pos = ChunkPos::at(client.old_position.x, client.old_position.z);
    let chunk_pos = ChunkPos::at(client.position.x, client.position.z);

    // Make sure the center chunk is set /before/ loading chunks.
    if old_chunk_pos != chunk_pos {
        send.append_packet(&SetCenterChunk {
            chunk_x: VarInt(chunk_pos.x),
            chunk_z: VarInt(chunk_pos.z),
        })?;
    }

    /*
    // Iterate over all visible chunks from the previous tick.
    if let Ok(old_instance) = instances.get(client.old_instance) {
        old_chunk_pos.try_for_each_in_view(client.old_view_distance, |pos| {
            if let Some(cell) = old_instance.cell(pos) {
                if let Some(chunk) = &cell.chunk {
                    if chunk.needs_reinit() {
                        todo!();

                        #[cfg(debug_assertions)]
                        client.loaded_chunks.insert(pos);
                    } else {
                        todo!();
                    }
                } else if cell.chunk_removed {
                    send.append_packet(&UnloadChunk {
                        chunk_x: pos.x,
                        chunk_z: pos.z,
                    })?;
                }
            }
        })?;
    }*/

    if client.is_new {
        // This closes the "downloading terrain" screen.
        // Send this after the initial chunks are loaded.
        send.append_packet(&SetDefaultSpawnPosition {
            position: BlockPos::at(client.position),
            angle: client.yaw,
        })?;
    }

    client.old_instance = client.instance;
    client.old_position = client.position;
    client.old_view_distance = client.view_distance;
    // TODO: clear client player data?

    send.flush().context("failed to flush packet queue")?;

    Ok(())
}
*/
