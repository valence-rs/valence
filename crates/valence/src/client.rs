use std::borrow::Cow;
use std::net::IpAddr;
use std::num::Wrapping;
use std::time::Instant;

use anyhow::{bail, Context};
use bevy_ecs::prelude::*;
use bytes::BytesMut;
use glam::{DVec3, Vec3};
use tracing::warn;
use uuid::Uuid;
use valence_protocol::packet::s2c::particle::Particle;
use valence_protocol::packet::s2c::play::{PlayerActionResponseS2c, DeathMessageS2c, DisconnectS2c, EntityStatusS2c, GameStateChangeS2c, KeepAliveS2c, GameJoinS2c, ParticleS2c, CustomPayloadS2c, EntitiesDestroyS2c, ResourcePackSendS2c, PlayerRespawnS2c, OverlayMessageS2c, ChunkRenderDistanceCenterS2c, PlayerSpawnPositionS2c, EntityTrackerUpdateS2c, EntityVelocityUpdateS2c, ChunkLoadDistanceS2c, SubtitleS2c, TitleFadeS2c, TitleS2c, PlaySoundS2c, PlayerPositionLookS2c, GameMessageS2c, UnloadChunkS2c};
use valence_protocol::types::{
    GameEventKind, GameMode, GlobalPos, Property, SoundCategory, SyncPlayerPosLookFlags,
};
use valence_protocol::{
    BlockPos, EncodePacket, Ident, ItemStack, PacketDecoder, PacketEncoder, RawBytes, Sound, Text,
    Username, VarInt,
};

use crate::dimension::DimensionId;
use crate::entity::data::Player;
use crate::entity::{velocity_to_packet_units, EntityStatus, McEntity};
use crate::instance::Instance;
use crate::packet::WritePacket;
use crate::server::{NewClientInfo, Server};
use crate::view::{ChunkPos, ChunkView};
use crate::{Despawned, NULL_ENTITY};

pub mod event;

/// Represents a client connected to the server. Used to send and receive
/// packets from the client.
#[derive(Component)]
pub struct Client {
    conn: Box<dyn ClientConnection>,
    enc: PacketEncoder,
    dec: PacketDecoder,
    scratch: Vec<u8>,
    is_disconnected: bool,
    username: Username<String>,
    uuid: Uuid,
    ip: IpAddr,
    properties: Vec<Property>,
    instance: Entity,
    old_instance: Entity,
    position: DVec3,
    old_position: DVec3,
    position_modified: bool,
    yaw: f32,
    yaw_modified: bool,
    pitch: f32,
    pitch_modified: bool,
    on_ground: bool,
    game_mode: GameMode,
    op_level: u8,
    block_change_sequence: i32,
    // TODO: make this a component and default to the self-entity's player data?
    player_data: Player,
    view_distance: u8,
    old_view_distance: u8,
    death_location: Option<(DimensionId, BlockPos)>,
    entities_to_despawn: Vec<VarInt>,
    got_keepalive: bool,
    last_keepalive_id: u64,
    keepalive_sent_time: Instant,
    ping: i32,
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
    /// The item that the client thinks it's holding under the mouse
    /// cursor.
    pub(crate) cursor_item: Option<ItemStack>,
    pub(crate) cursor_item_modified: bool,
    /// The current window ID. Incremented when inventories are opened.
    pub(crate) window_id: u8,
    pub(crate) inventory_state_id: Wrapping<i32>,
    /// Tracks what slots have been modified by this client in this tick, so we
    /// don't need to send updates for them.
    pub(crate) inventory_slots_modified: u64,
    pub(crate) held_item_slot: u16,
}

pub trait ClientConnection: Send + Sync + 'static {
    fn try_send(&mut self, bytes: BytesMut) -> anyhow::Result<()>;
    fn try_recv(&mut self) -> anyhow::Result<BytesMut>;
}

impl Client {
    pub(crate) fn new(
        info: NewClientInfo,
        conn: Box<dyn ClientConnection>,
        enc: PacketEncoder,
        dec: PacketDecoder,
    ) -> Self {
        Self {
            conn,
            enc,
            dec,
            scratch: vec![],
            is_disconnected: false,
            username: info.username,
            uuid: info.uuid,
            ip: info.ip,
            properties: info.properties,
            instance: NULL_ENTITY,
            old_instance: NULL_ENTITY,
            position: DVec3::ZERO,
            old_position: DVec3::ZERO,
            position_modified: true,
            yaw: 0.0,
            yaw_modified: true,
            pitch: 0.0,
            pitch_modified: true,
            on_ground: false,
            game_mode: GameMode::default(),
            op_level: 0,
            block_change_sequence: 0,
            player_data: Player::new(),
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
            keepalive_sent_time: Instant::now(),
            ping: -1,
            teleport_id_counter: 0,
            pending_teleports: 0,
            cursor_item: None,
            cursor_item_modified: false,
            window_id: 0,
            inventory_state_id: Wrapping(0),
            inventory_slots_modified: 0,
            held_item_slot: 0,
        }
    }

    pub(crate) fn is_new(&self) -> bool {
        self.is_new
    }

    /// Attempts to write a play packet into this client's packet buffer. The
    /// packet will be sent at the end of the tick.
    ///
    /// If encoding the packet fails, the client is disconnected. Has no
    /// effect if the client is already disconnected.
    pub fn write_packet<P>(&mut self, pkt: &P)
    where
        P: EncodePacket + ?Sized,
    {
        self.enc.write_packet(pkt);
    }

    /// Writes arbitrary bytes to this client's packet buffer. The packet data
    /// must be properly compressed for the current compression threshold but
    /// never encrypted.
    ///
    /// Don't use this function unless you know what you're doing. Consider
    /// using [`write_packet`] instead.
    ///
    /// [`write_packet`]: Self::write_packet
    #[inline]
    pub fn write_packet_bytes(&mut self, bytes: &[u8]) {
        self.enc.append_bytes(bytes);
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

    /// Gets the properties from this client's game profile.
    pub fn properties(&self) -> &[Property] {
        &self.properties
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

    pub fn set_position(&mut self, pos: impl Into<DVec3>) {
        self.position = pos.into();
        self.position_modified = true;
    }

    /// Returns the position this client was in at the end of the previous tick.
    pub fn old_position(&self) -> DVec3 {
        self.old_position
    }

    /// Gets a [`ChunkView`] representing the chunks this client can see.
    pub fn view(&self) -> ChunkView {
        ChunkView::new(ChunkPos::from_dvec3(self.position), self.view_distance)
    }

    pub fn old_view(&self) -> ChunkView {
        ChunkView::new(
            ChunkPos::from_dvec3(self.old_position),
            self.old_view_distance,
        )
    }

    pub fn set_velocity(&mut self, velocity: impl Into<Vec3>) {
        self.enc.write_packet(&EntityVelocityUpdateS2c {
            entity_id: VarInt(0),
            velocity: velocity_to_packet_units(velocity.into()),
        });
    }

    /// Gets this client's yaw (in degrees).
    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    /// Sets this client's yaw (in degrees).
    pub fn set_yaw(&mut self, yaw: f32) {
        self.yaw = yaw;
        self.yaw_modified = true;
    }

    /// Gets this client's pitch (in degrees).
    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// Sets this client's pitch (in degrees).
    pub fn set_pitch(&mut self, pitch: f32) {
        self.pitch = pitch;
        self.pitch_modified = true;
    }

    /// Whether or not the client reports that it is currently on the ground.
    pub fn on_ground(&self) -> bool {
        self.on_ground
    }

    /// Kills the client and shows `message` on the death screen. If an entity
    /// killed the player, you should supply it as `killer`.
    pub fn kill(&mut self, killer: Option<&McEntity>, message: impl Into<Text>) {
        self.write_packet(&DeathMessageS2c {
            player_id: VarInt(0),
            entity_id: killer.map_or(-1, |k| k.protocol_id()),
            message: message.into().into(),
        });
    }

    /// Respawns client. Optionally can roll the credits before respawning.
    pub fn win_game(&mut self, show_credits: bool) {
        self.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::WinGame,
            value: if show_credits { 1.0 } else { 0.0 },
        });
    }

    pub fn has_respawn_screen(&self) -> bool {
        self.has_respawn_screen
    }

    /// Sets whether respawn screen should be displayed after client's death.
    pub fn set_respawn_screen(&mut self, enable: bool) {
        if self.has_respawn_screen != enable {
            self.has_respawn_screen = enable;

            if !self.is_new {
                self.write_packet(&GameStateChangeS2c {
                    kind: GameEventKind::EnableRespawnScreen,
                    value: if enable { 0.0 } else { 1.0 },
                });
            }
        }
    }

    /// Gets whether or not the client thinks it's on a superflat world.
    ///
    /// Modifies how the skybox is rendered.
    pub fn is_flat(&self) -> bool {
        self.is_flat
    }

    /// Sets whether or not the client thinks it's on a superflat world.
    ///
    /// Modifies how the skybox is rendered.
    pub fn set_flat(&mut self, flat: bool) {
        self.is_flat = flat;
    }

    /// The current view distance of this client measured in chunks. The client
    /// will not be able to see chunks and entities past this distance.
    ///
    /// The result is in `2..=32`.
    pub fn view_distance(&self) -> u8 {
        self.view_distance
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
                self.write_packet(&GameStateChangeS2c {
                    kind: GameEventKind::ChangeGameMode,
                    value: game_mode as i32 as f32,
                });
            }
        }
    }

    /// Sets the client's OP level.
    pub fn set_op_level(&mut self, op_level: u8) {
        self.op_level = op_level;

        if op_level > 4 {
            return;
        }

        self.write_packet(&EntityStatusS2c {
            entity_id: 0,
            entity_status: 24 + op_level,
        });
    }

    /// Gets the client's OP level.
    pub fn op_level(&self) -> u8 {
        self.op_level
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

    pub fn trigger_status(&mut self, status: EntityStatusS2c) {
        self.write_packet(&EntityStatusS2c {
            entity_id: 0,
            entity_status: status as u8,
        });
    }

    pub fn ping(&self) -> i32 {
        self.ping
    }

    /// The item that the client thinks it's holding under the mouse
    /// cursor. Only relevant when the client has an open inventory.
    pub fn cursor_item(&self) -> Option<&ItemStack> {
        self.cursor_item.as_ref()
    }

    pub fn replace_cursor_item(&mut self, item: impl Into<Option<ItemStack>>) -> Option<ItemStack> {
        let new = item.into();
        if self.cursor_item != new {
            self.cursor_item_modified = true;
        }

        std::mem::replace(&mut self.cursor_item, new)
    }

    pub fn player(&self) -> &Player {
        &self.player_data
    }

    pub fn player_mut(&mut self) -> &mut Player {
        &mut self.player_data
    }

    /// Sends a system message to the player which is visible in the chat. The
    /// message is only visible to this client.
    pub fn send_message(&mut self, msg: impl Into<Text>) {
        self.write_packet(&GameMessageS2c {
            chat: msg.into().into(),
            overlay: false,
        });
    }

    pub fn send_plugin_message(&mut self, channel: Ident<&str>, data: &[u8]) {
        self.write_packet(&CustomPayloadS2c {
            channel,
            data: RawBytes(data),
        });
    }

    /// Get the slot id in the player's inventory that the client says it's
    /// holding.
    pub fn held_item_slot(&self) -> u16 {
        self.held_item_slot
    }

    /// Kick the client with the given reason.
    pub fn kick(&mut self, reason: impl Into<Text>) {
        self.write_packet(&DisconnectS2c {
            reason: reason.into().into(),
        });
        self.is_disconnected = true;
    }

    /// Requests that the client download and enable a resource pack.
    ///
    /// # Arguments
    /// * `url` - The URL of the resource pack file.
    /// * `hash` - The SHA-1 hash of the resource pack file. Any value other
    ///   than a 40-character hexadecimal string is ignored by the client.
    /// * `forced` - Whether a client should be kicked from the server upon
    ///   declining the pack (this is enforced client-side)
    /// * `prompt_message` - A message to be displayed with the resource pack
    ///   dialog.
    pub fn set_resource_pack(
        &mut self,
        url: &str,
        hash: &str,
        forced: bool,
        prompt_message: Option<Text>,
    ) {
        self.write_packet(&ResourcePackSendS2c {
            url,
            hash,
            forced,
            prompt_message: prompt_message.map(|t| t.into()),
        });
    }

    /// Sets the title this client sees.
    ///
    /// A title is a large piece of text displayed in the center of the screen
    /// which may also include a subtitle underneath it. The title can be
    /// configured to fade in and out using the [`TitleFadeS2c`]
    /// struct.
    pub fn set_title(
        &mut self,
        title: impl Into<Text>,
        subtitle: impl Into<Text>,
        animation: impl Into<Option<TitleFadeS2c>>,
    ) {
        let title = title.into().into();
        let subtitle = subtitle.into();

        self.write_packet(&TitleS2c { title_text: title });

        if !subtitle.is_empty() {
            self.write_packet(&SubtitleS2c {
                subtitle_text: subtitle.into(),
            });
        }

        if let Some(anim) = animation.into() {
            self.write_packet(&anim);
        }
    }

    /// Sets the action bar for this client.
    ///
    /// The action bar is a small piece of text displayed at the bottom of the
    /// screen, above the hotbar.
    pub fn set_action_bar(&mut self, text: impl Into<Text>) {
        self.write_packet(&OverlayMessageS2c {
            action_bar_text: text.into().into(),
        });
    }

    /// Puts a particle effect at the given position, only for this client.
    ///
    /// If you want to show a particle effect to all players, use
    /// [`Instance::play_particle`]
    ///
    /// [`Instance::play_particle`]: crate::instance::Instance::play_particle
    pub fn play_particle(
        &mut self,
        particle: &Particle,
        long_distance: bool,
        position: impl Into<DVec3>,
        offset: impl Into<Vec3>,
        max_speed: f32,
        count: i32,
    ) {
        self.write_packet(&ParticleS2c {
            particle: particle.clone(),
            long_distance,
            position: position.into().into(),
            offset: offset.into().into(),
            max_speed,
            count,
        })
    }

    /// Plays a sound effect at the given position, only for this client.
    ///
    /// If you want to play a sound effect to all players, use
    /// [`Instance::play_sound`]
    ///
    /// [`Instance::play_sound`]: crate::instance::Instance::play_sound
    pub fn play_sound(
        &mut self,
        sound: Sound,
        category: SoundCategory,
        position: impl Into<DVec3>,
        volume: f32,
        pitch: f32,
    ) {
        let position = position.into();

        self.write_packet(&PlaySoundS2c {
            id: sound.to_id(),
            category,
            position: (position * 8.0).as_ivec3().into(),
            volume,
            pitch,
            seed: rand::random(),
        });
    }
}

impl WritePacket for Client {
    fn write_packet<P>(&mut self, packet: &P)
    where
        P: EncodePacket + ?Sized,
    {
        self.enc.write_packet(packet)
    }

    fn write_packet_bytes(&mut self, bytes: &[u8]) {
        self.enc.write_packet_bytes(bytes)
    }
}

/// A system for adding [`Despawned`] components to disconnected clients.
pub fn despawn_disconnected_clients(mut commands: Commands, clients: Query<(Entity, &Client)>) {
    for (entity, client) in &clients {
        if client.is_disconnected() {
            commands.entity(entity).insert(Despawned);
        }
    }
}

pub(crate) fn update_clients(
    server: Res<Server>,
    mut clients: Query<(Entity, &mut Client, Option<&McEntity>)>,
    instances: Query<&Instance>,
    entities: Query<&McEntity>,
) {
    // TODO: what batch size to use?
    clients.par_for_each_mut(16, |(entity_id, mut client, self_entity)| {
        if !client.is_disconnected() {
            if let Err(e) = update_one_client(
                &mut client,
                self_entity,
                entity_id,
                &instances,
                &entities,
                &server,
            ) {
                client.write_packet(&DisconnectS2c {
                    reason: Text::from("").into(),
                });
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
    });
}

#[inline]
fn update_one_client(
    client: &mut Client,
    _self_entity: Option<&McEntity>,
    _self_id: Entity,
    instances: &Query<&Instance>,
    entities: &Query<&McEntity>,
    server: &Server,
) -> anyhow::Result<()> {
    let Ok(instance) = instances.get(client.instance) else {
        bail!("client is in a nonexistent instance");
    };

    // Send the login (play) packet and other initial packets. We defer this until
    // now so that the user can set the client's initial location, game
    // mode, etc.
    if client.is_new {
        client.needs_respawn = false;

        let dimension_names = server
            .dimensions()
            .map(|(_, dim)| dim.name.as_str_ident())
            .collect();

        let dimension_name = server.dimension(instance.dimension()).name.as_str_ident();

        let last_death_location = client.death_location.map(|(id, pos)| GlobalPos {
            dimension_name: server.dimension(id).name.as_str_ident(),
            position: pos,
        });

        // The login packet is prepended so that it is sent before all the other
        // packets. Some packets don't work correctly when sent before the login packet,
        // which is why we're doing this.
        client.enc.prepend_packet(&GameJoinS2c {
            entity_id: 0, // ID 0 is reserved for clients.
            is_hardcore: client.is_hardcore,
            game_mode: client.game_mode,
            previous_game_mode: -1,
            dimension_names,
            registry_codec: Cow::Borrowed(server.registry_codec()),
            dimension_type_name: dimension_name,
            dimension_name,
            hashed_seed: 42,
            max_players: VarInt(0), // Unused
            view_distance: VarInt(client.view_distance() as i32),
            simulation_distance: VarInt(16),
            reduced_debug_info: false,
            enable_respawn_screen: client.has_respawn_screen,
            is_debug: false,
            is_flat: client.is_flat,
            last_death_location,
        })?;

        /*
        // TODO: enable all the features?
        send.append_packet(&FeatureFlags {
            features: vec![Ident::new("vanilla").unwrap()],
        })?;
        */
    } else {
        if client.view_distance != client.old_view_distance {
            // Change the render distance fog.
            client.enc.append_packet(&ChunkLoadDistanceS2c {
                view_distance: VarInt(client.view_distance.into()),
            })?;
        }

        if client.needs_respawn {
            client.needs_respawn = false;

            let dimension_name = server.dimension(instance.dimension()).name.as_str_ident();

            let last_death_location = client.death_location.map(|(id, pos)| GlobalPos {
                dimension_name: server.dimension(id).name.as_str_ident(),
                position: pos,
            });

            client.enc.append_packet(&PlayerRespawnS2c {
                dimension_type_name: dimension_name,
                dimension_name,
                hashed_seed: 0,
                game_mode: client.game_mode,
                previous_game_mode: -1,
                is_debug: false,
                is_flat: client.is_flat,
                copy_metadata: true,
                last_death_location,
            })?;
        }
    }

    // Check if it's time to send another keepalive.
    if server.current_tick() % (server.tps() * 10) == 0 {
        if client.got_keepalive {
            let id = rand::random();
            client.enc.write_packet(&KeepAliveS2c { id });

            client.got_keepalive = false;
            client.last_keepalive_id = id;
            client.keepalive_sent_time = Instant::now();
        } else {
            bail!("timed out (no keepalive response)");
        }
    }

    // Send instance-wide packet data.
    client.enc.append_bytes(&instance.packet_buf);

    let old_view = client.old_view();
    let view = client.view();

    // Make sure the center chunk is set before loading chunks!
    if old_view.pos != view.pos {
        // TODO: does the client initialize the center chunk to (0, 0)?
        client.enc.write_packet(&ChunkRenderDistanceCenterS2c {
            chunk_x: VarInt(view.pos.x),
            chunk_z: VarInt(view.pos.z),
        });
    }

    // Iterate over all visible chunks from the previous tick.
    if let Ok(old_instance) = instances.get(client.old_instance) {
        old_view.for_each(|pos| {
            if let Some(cell) = old_instance.partition.get(&pos) {
                if cell.chunk_removed && cell.chunk.is_none() {
                    // Chunk was previously loaded and is now deleted.
                    client.enc.write_packet(&UnloadChunkS2c {
                        chunk_x: pos.x,
                        chunk_z: pos.z,
                    });
                }

                if let Some(chunk) = &cell.chunk {
                    chunk.mark_viewed();
                }

                // Send entity spawn packets for entities entering the client's view.
                for &(id, src_pos) in &cell.incoming {
                    if src_pos.map_or(true, |p| !old_view.contains(p)) {
                        // The incoming entity originated from outside the view distance, so it
                        // must be spawned.
                        if let Ok(entity) = entities.get(id) {
                            // Spawn the entity at the old position so that later relative entity
                            // movement packets will not set the entity to the wrong position.
                            entity.write_init_packets(
                                &mut client.enc,
                                entity.old_position(),
                                &mut client.scratch,
                            );
                        }
                    }
                }

                // Send entity despawn packets for entities exiting the client's view.
                for &(id, dest_pos) in &cell.outgoing {
                    if dest_pos.map_or(true, |p| !old_view.contains(p)) {
                        // The outgoing entity moved outside the view distance, so it must be
                        // despawned.
                        if let Ok(entity) = entities.get(id) {
                            client
                                .entities_to_despawn
                                .push(VarInt(entity.protocol_id()));
                        }
                    }
                }

                // Send all data in the chunk's packet buffer to this client. This will update
                // entities in the cell, spawn or update the chunk in the cell, or send any
                // other packet data that was added here by users.
                client.enc.append_bytes(&cell.packet_buf);
            }
        });
    }

    // Was the client's instance changed?
    if client.old_instance != client.instance {
        if let Ok(old_instance) = instances.get(client.old_instance) {
            // TODO: only send unload packets when old dimension == new dimension, since the
            //       client will do the unloading for us in that case?

            // Unload all chunks and entities in the old view.
            old_view.for_each(|pos| {
                if let Some(cell) = old_instance.partition.get(&pos) {
                    // Unload the chunk at this cell if it was loaded.
                    if cell.chunk.is_some() {
                        client.enc.write_packet(&UnloadChunkS2c {
                            chunk_x: pos.x,
                            chunk_z: pos.z,
                        });
                    }

                    // Unload all the entities in the cell.
                    for &id in &cell.entities {
                        if let Ok(entity) = entities.get(id) {
                            client
                                .entities_to_despawn
                                .push(VarInt(entity.protocol_id()));
                        }
                    }
                }
            });
        }

        // Load all chunks and entities in new view.
        view.for_each(|pos| {
            if let Some(cell) = instance.partition.get(&pos) {
                // Load the chunk at this cell if there is one.
                if let Some(chunk) = &cell.chunk {
                    chunk.write_init_packets(
                        &instance.info,
                        pos,
                        &mut client.enc,
                        &mut client.scratch,
                    );

                    chunk.mark_viewed();
                }

                // Load all the entities in this cell.
                for &id in &cell.entities {
                    if let Ok(entity) = entities.get(id) {
                        entity.write_init_packets(
                            &mut client.enc,
                            entity.position(),
                            &mut client.scratch,
                        );
                    }
                }
            }
        });
    } else if old_view != view {
        // Client changed their view without changing the instance.

        // Unload chunks and entities in the old view and load chunks and entities in
        // the new view. We don't need to do any work where the old and new view
        // overlap.
        old_view.diff_for_each(view, |pos| {
            if let Some(cell) = instance.partition.get(&pos) {
                // Unload the chunk at this cell if it was loaded.
                if cell.chunk.is_some() {
                    client.enc.write_packet(&UnloadChunkS2c {
                        chunk_x: pos.x,
                        chunk_z: pos.z,
                    });
                }

                // Unload all the entities in the cell.
                for &id in &cell.entities {
                    if let Ok(entity) = entities.get(id) {
                        client
                            .entities_to_despawn
                            .push(VarInt(entity.protocol_id()));
                    }
                }
            }
        });

        view.diff_for_each(old_view, |pos| {
            if let Some(cell) = instance.partition.get(&pos) {
                // Load the chunk at this cell if there is one.
                if let Some(chunk) = &cell.chunk {
                    chunk.write_init_packets(
                        &instance.info,
                        pos,
                        &mut client.enc,
                        &mut client.scratch,
                    );

                    chunk.mark_viewed();
                }

                // Load all the entities in this cell.
                for &id in &cell.entities {
                    if let Ok(entity) = entities.get(id) {
                        entity.write_init_packets(
                            &mut client.enc,
                            entity.position(),
                            &mut client.scratch,
                        );
                    }
                }
            }
        });
    }

    // Despawn all the entities that are queued to be despawned.
    if !client.entities_to_despawn.is_empty() {
        client.enc.append_packet(&EntitiesDestroyS2c {
            entity_ids: Cow::Borrowed(&client.entities_to_despawn),
        })?;

        client.entities_to_despawn.clear();
    }

    // Teleport the client. Do this after chunk packets are sent so the client does
    // not accidentally pass through blocks.
    if client.position_modified || client.yaw_modified || client.pitch_modified {
        let flags = SyncPlayerPosLookFlags::new()
            .with_x(!client.position_modified)
            .with_y(!client.position_modified)
            .with_z(!client.position_modified)
            .with_y_rot(!client.yaw_modified)
            .with_x_rot(!client.pitch_modified);

        client.enc.write_packet(&PlayerPositionLookS2c {
            position: if client.position_modified {
                client.position.to_array()
            } else {
                [0.0; 3]
            },
            yaw: if client.yaw_modified { client.yaw } else { 0.0 },
            pitch: if client.pitch_modified {
                client.pitch
            } else {
                0.0
            },
            flags,
            teleport_id: VarInt(client.teleport_id_counter as i32),
            dismount_vehicle: false,
        });

        client.pending_teleports = client.pending_teleports.wrapping_add(1);
        client.teleport_id_counter = client.teleport_id_counter.wrapping_add(1);

        client.position_modified = false;
        client.yaw_modified = false;
        client.pitch_modified = false;
    }

    // This closes the "downloading terrain" screen.
    // Send this after the initial chunks are loaded.
    if client.is_new {
        client.enc.write_packet(&PlayerSpawnPositionS2c {
            position: BlockPos::at(client.position),
            angle: client.yaw,
        });
    }

    // Update the client's own player metadata.
    client.scratch.clear();
    client.player_data.updated_tracked_data(&mut client.scratch);
    if !client.scratch.is_empty() {
        client.player_data.clear_modifications();

        client.scratch.push(0xff);

        client.enc.write_packet(&EntityTrackerUpdateS2c {
            entity_id: VarInt(0),
            metadata: RawBytes(&client.scratch),
        });
    }

    // Acknowledge broken/placed blocks.
    if client.block_change_sequence != 0 {
        client.enc.write_packet(&PlayerActionResponseS2c {
            sequence: VarInt(client.block_change_sequence),
        });

        client.block_change_sequence = 0;
    }

    client.old_instance = client.instance;
    client.old_position = client.position;
    client.old_view_distance = client.view_distance;

    client
        .conn
        .try_send(client.enc.take())
        .context("failed to flush packet queue")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use bevy_app::App;
    use valence_protocol::packets::s2c::play::ChunkDataAndUpdateLight;
    use valence_protocol::packets::S2cPlayPacket;

    use super::*;
    use crate::instance::Chunk;
    use crate::unit_test::util::scenario_single_client;

    #[test]
    fn client_chunk_view_change() {
        let mut app = App::new();

        let (client_ent, mut client_helper) = scenario_single_client(&mut app);

        let mut instance = app
            .world
            .query::<&mut Instance>()
            .single_mut(&mut app.world);

        for z in -15..15 {
            for x in -15..15 {
                instance.insert_chunk([x, z], Chunk::default());
            }
        }

        let mut client = app.world.get_mut::<Client>(client_ent).unwrap();

        client.set_position([8.0, 0.0, 8.0]);
        client.set_view_distance(6);

        // Tick
        app.update();
        let mut client = app.world.get_mut::<Client>(client_ent).unwrap();

        let mut loaded_chunks = BTreeSet::new();

        for pkt in client_helper.collect_sent().unwrap() {
            if let S2cPlayPacket::ChunkDataAndUpdateLight(ChunkDataAndUpdateLight {
                chunk_x,
                chunk_z,
                ..
            }) = pkt
            {
                assert!(
                    loaded_chunks.insert(ChunkPos::new(chunk_x, chunk_z)),
                    "({chunk_x}, {chunk_z})"
                );
            }
        }

        for pos in client.view().iter() {
            assert!(loaded_chunks.contains(&pos), "{pos:?}");
        }

        assert!(!loaded_chunks.is_empty());

        // Move the client to the adjacent chunk.
        client.set_position([24.0, 0.0, 24.0]);

        // Tick
        app.update();
        let client = app.world.get_mut::<Client>(client_ent).unwrap();

        for pkt in client_helper.collect_sent().unwrap() {
            match pkt {
                S2cPlayPacket::ChunkDataAndUpdateLight(ChunkDataAndUpdateLight {
                    chunk_x,
                    chunk_z,
                    ..
                }) => {
                    assert!(
                        loaded_chunks.insert(ChunkPos::new(chunk_x, chunk_z)),
                        "({chunk_x}, {chunk_z})"
                    );
                }
                S2cPlayPacket::UnloadChunk(UnloadChunkS2c { chunk_x, chunk_z }) => {
                    assert!(
                        loaded_chunks.remove(&ChunkPos::new(chunk_x, chunk_z)),
                        "({chunk_x}, {chunk_z})"
                    );
                }
                _ => {}
            }
        }

        for pos in client.view().iter() {
            assert!(loaded_chunks.contains(&pos), "{pos:?}");
        }
    }
}
