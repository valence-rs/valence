use std::borrow::Cow;
use std::net::IpAddr;
use std::num::Wrapping;
use std::time::Instant;

use bevy_ecs::prelude::*;
use bevy_ecs::query::WorldQuery;
use bevy_ecs::schedule::SystemConfigs;
use bevy_ecs::system::Command;
use bytes::BytesMut;
use glam::{DVec3, Vec3};
use rand::Rng;
use tracing::warn;
use valence_protocol::block_pos::BlockPos;
use valence_protocol::codec::{PacketDecoder, PacketEncoder};
use valence_protocol::ident::Ident;
use valence_protocol::item::ItemStack;
use valence_protocol::packet::s2c::play::game_state_change::GameEventKind;
use valence_protocol::packet::s2c::play::particle::Particle;
use valence_protocol::packet::s2c::play::player_position_look::Flags as PlayerPositionLookFlags;
use valence_protocol::packet::s2c::play::{
    ChunkLoadDistanceS2c, ChunkRenderDistanceCenterS2c, CustomPayloadS2c, DeathMessageS2c,
    DisconnectS2c, EntitiesDestroyS2c, EntityStatusS2c, EntityTrackerUpdateS2c,
    EntityVelocityUpdateS2c, GameJoinS2c, GameMessageS2c, GameStateChangeS2c, KeepAliveS2c,
    OverlayMessageS2c, ParticleS2c, PlaySoundS2c, PlayerActionResponseS2c, PlayerPositionLookS2c,
    PlayerRespawnS2c, PlayerSpawnPositionS2c, ResourcePackSendS2c, SubtitleS2c, TitleFadeS2c,
    TitleS2c, UnloadChunkS2c,
};
use valence_protocol::sound::Sound;
use valence_protocol::text::Text;
use valence_protocol::types::{GlobalPos, SoundCategory};
use valence_protocol::var_int::VarInt;
use valence_protocol::Packet;

use crate::component::{
    Despawned, GameMode, Location, OldLocation, OldPosition, OnGround, Ping, Pitch, Position,
    Properties, UniqueId, Username, Yaw,
};
use crate::dimension::DimensionId;
use crate::entity::{velocity_to_packet_units, EntityStatus, McEntity, TrackedData};
use crate::instance::Instance;
use crate::inventory::{Inventory, InventoryKind};
use crate::packet::WritePacket;
use crate::server::{NewClientInfo, Server};
use crate::view::{ChunkPos, ChunkView};

pub mod event;

/// The bundle of components needed for clients to function. All components are
/// required unless otherwise stated.
#[derive(Bundle)]
pub(crate) struct ClientBundle {
    client: Client,
    entity_remove_buffer: EntityRemoveBuffer,
    username: Username,
    uuid: UniqueId,
    ip: Ip,
    properties: Properties,
    pub location: Location,
    pub old_location: OldLocation,
    position: Position,
    old_position: OldPosition,
    yaw: Yaw,
    pitch: Pitch,
    on_ground: OnGround,
    compass_pos: CompassPos,
    game_mode: GameMode,
    op_level: OpLevel,
    player_action_sequence: PlayerActionSequence,
    view_distance: ViewDistance,
    old_view_distance: OldViewDistance,
    death_location: DeathLocation,
    keepalive_state: KeepaliveState,
    ping: Ping,
    is_hardcore: IsHardcore,
    prev_game_mode: PrevGameMode,
    hashed_seed: HashedSeed,
    reduced_debug_info: ReducedDebugInfo,
    has_respawn_screen: HasRespawnScreen,
    is_debug: IsDebug,
    is_flat: IsFlat,
    teleport_state: TeleportState,
    cursor_item: CursorItem,
    player_inventory_state: PlayerInventoryState,
    inventory: Inventory,
}

impl ClientBundle {
    pub(crate) fn new(
        info: NewClientInfo,
        conn: Box<dyn ClientConnection>,
        enc: PacketEncoder,
        dec: PacketDecoder,
    ) -> Self {
        Self {
            client: Client {
                conn,
                enc,
                dec,
                scratch: Vec::new(),
            },
            entity_remove_buffer: EntityRemoveBuffer(vec![]),
            username: Username(info.username),
            uuid: UniqueId(info.uuid),
            ip: Ip(info.ip),
            properties: Properties(info.properties),
            location: Location::default(),
            old_location: OldLocation::default(),
            position: Position::default(),
            old_position: OldPosition::default(),
            yaw: Yaw::default(),
            pitch: Pitch::default(),
            on_ground: OnGround::default(),
            compass_pos: CompassPos::default(),
            game_mode: GameMode::default(),
            op_level: OpLevel::default(),
            player_action_sequence: PlayerActionSequence(0),
            view_distance: ViewDistance::default(),
            old_view_distance: OldViewDistance(2),
            death_location: DeathLocation::default(),
            keepalive_state: KeepaliveState {
                got_keepalive: true,
                last_keepalive_id: 0,
                keepalive_sent_time: Instant::now(),
            },
            ping: Ping::default(),
            teleport_state: TeleportState {
                teleport_id_counter: 0,
                pending_teleports: 0,
            },
            is_hardcore: IsHardcore::default(),
            is_flat: IsFlat::default(),
            has_respawn_screen: HasRespawnScreen::default(),
            cursor_item: CursorItem::default(),
            player_inventory_state: PlayerInventoryState {
                window_id: 0,
                state_id: Wrapping(0),
                slots_changed: 0,
                // First slot of the hotbar.
                held_item_slot: 36,
            },
            inventory: Inventory::new(InventoryKind::Player),
            prev_game_mode: PrevGameMode::default(),
            hashed_seed: HashedSeed::default(),
            reduced_debug_info: ReducedDebugInfo::default(),
            is_debug: IsDebug::default(),
        }
    }
}

/// The main client component. Contains the underlying network connection and
/// packet buffer.
///
/// The component is removed when the client is disconnected. You are allowed to
/// remove the component yourself.
#[derive(Component)]
pub struct Client {
    conn: Box<dyn ClientConnection>,
    enc: PacketEncoder,
    dec: PacketDecoder,
    /// Scratch buffer.
    // TODO: move this to separate component.
    scratch: Vec<u8>,
}

pub(crate) trait ClientConnection: Send + Sync + 'static {
    fn try_send(&mut self, bytes: BytesMut) -> anyhow::Result<()>;
    fn try_recv(&mut self) -> anyhow::Result<BytesMut>;
}

impl Drop for Client {
    fn drop(&mut self) {
        _ = self.flush_packets();
    }
}

/// Writes packets into this client's packet buffer. The buffer is flushed at
/// the end of the tick.
impl WritePacket for Client {
    fn write_packet<'a>(&mut self, packet: &impl Packet<'a>) {
        self.enc.write_packet(packet)
    }

    fn write_packet_bytes(&mut self, bytes: &[u8]) {
        self.enc.write_packet_bytes(bytes)
    }
}

impl Client {
    /// Flushes the packet queue to the underlying connection.
    ///
    /// This is called automatically at the end of the tick and when the client
    /// is dropped. Unless you're in a hurry, there's usually no reason to
    /// call this method yourself.
    ///
    /// Returns an error if flushing was unsuccessful.
    pub fn flush_packets(&mut self) -> anyhow::Result<()> {
        self.conn.try_send(self.enc.take())
    }

    /// Sends a system message to the player which is visible in the chat. The
    /// message is only visible to this client.
    pub fn send_message(&mut self, msg: impl Into<Text>) {
        self.write_packet(&GameMessageS2c {
            chat: msg.into().into(),
            overlay: false,
        });
    }

    pub fn send_custom_payload(&mut self, channel: Ident<&str>, data: &[u8]) {
        self.write_packet(&CustomPayloadS2c {
            channel,
            data: data.into(),
        });
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
    /// [`Instance::play_particle`]: Instance::play_particle
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
            particle: Cow::Borrowed(particle),
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
    /// [`Instance::play_sound`]: Instance::play_sound
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

    /// `velocity` is in m/s.
    pub fn set_velocity(&mut self, velocity: impl Into<Vec3>) {
        self.write_packet(&EntityVelocityUpdateS2c {
            entity_id: VarInt(0),
            velocity: velocity_to_packet_units(velocity.into()),
        });
    }

    /// Triggers an [`EntityStatus`].
    ///
    /// The status is only visible to this client.
    pub fn trigger_status(&mut self, status: EntityStatus) {
        self.write_packet(&EntityStatusS2c {
            entity_id: 0,
            entity_status: status as u8,
        });
    }
}

/// A [`Command`] to disconnect a [`Client`] with a displayed reason.
#[derive(Clone, PartialEq, Debug)]
pub struct DisconnectClient {
    pub client: Entity,
    pub reason: Text,
}

impl Command for DisconnectClient {
    fn write(self, world: &mut World) {
        if let Some(mut entity) = world.get_entity_mut(self.client) {
            if let Some(mut client) = entity.get_mut::<Client>() {
                client.write_packet(&DisconnectS2c {
                    reason: self.reason.into(),
                });

                entity.remove::<Client>();
            }
        }
    }
}

/// Contains a list of Minecraft entities that need to be despawned. Entity IDs
/// in this list will be despawned all at once at the end of the tick.
///
/// You should not need to use this directly under normal circumstances.
#[derive(Component, Debug)]
pub struct EntityRemoveBuffer(Vec<VarInt>);

impl EntityRemoveBuffer {
    pub fn push(&mut self, entity_id: i32) {
        debug_assert!(
            entity_id != 0,
            "removing entity with protocol ID 0 (which should be reserved for clients)"
        );

        debug_assert!(
            !self.0.contains(&VarInt(entity_id)),
            "removing entity ID {entity_id} multiple times in a single tick!"
        );

        self.0.push(VarInt(entity_id));
    }
}

#[derive(Component, Clone, PartialEq, Eq, Debug)]
pub struct Ip(pub IpAddr);

/// The position that regular compass items will point to.
#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct CompassPos(pub BlockPos);

#[derive(Component, Clone, PartialEq, Eq, Default, Debug)]
pub struct OpLevel(u8);

impl OpLevel {
    pub fn get(&self) -> u8 {
        self.0
    }

    /// Sets the op level. Value is clamped to `0..=3`.
    pub fn set(&mut self, lvl: u8) {
        self.0 = lvl.min(3);
    }
}

#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct PlayerActionSequence(i32);

#[derive(Component, Clone, PartialEq, Eq, Debug)]

pub struct ViewDistance(u8);

impl ViewDistance {
    pub fn get(&self) -> u8 {
        self.0
    }

    /// `dist` is clamped to `2..=32`.
    pub fn set(&mut self, dist: u8) {
        self.0 = dist.clamp(2, 32);
    }
}

impl Default for ViewDistance {
    fn default() -> Self {
        Self(2)
    }
}

/// The [`ViewDistance`] at the end of the previous tick. Automatically updated
/// as [`ViewDistance`] is changed.
#[derive(Component, Clone, PartialEq, Eq, Default, Debug)]

pub struct OldViewDistance(u8);

impl OldViewDistance {
    pub fn get(&self) -> u8 {
        self.0
    }
}

#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct DeathLocation(pub Option<(DimensionId, BlockPos)>);

#[derive(Component, Debug)]
pub struct KeepaliveState {
    got_keepalive: bool,
    last_keepalive_id: u64,
    keepalive_sent_time: Instant,
}

#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct IsHardcore(pub bool);

/// The initial previous gamemode. Used for the F3+F4 gamemode switcher.
#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct PrevGameMode(pub Option<GameMode>);

/// Hashed world seed used for biome noise.
#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct HashedSeed(pub u64);

#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct ReducedDebugInfo(pub bool);

#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct HasRespawnScreen(pub bool);

#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct IsDebug(pub bool);

/// Changes the perceived horizon line (used for superflat worlds).
#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct IsFlat(pub bool);

#[derive(Component, Debug)]
pub struct TeleportState {
    /// Counts up as teleports are made.
    teleport_id_counter: u32,
    /// The number of pending client teleports that have yet to receive a
    /// confirmation. Inbound client position packets should be ignored while
    /// this is nonzero.
    pending_teleports: u32,
}

impl TeleportState {
    pub fn teleport_id_counter(&self) -> u32 {
        self.teleport_id_counter
    }

    pub fn pending_teleports(&self) -> u32 {
        self.pending_teleports
    }
}

/// The item that the client thinks it's holding under the mouse
/// cursor.
#[derive(Component, Clone, PartialEq, Default, Debug)]
pub struct CursorItem(pub Option<ItemStack>);

// TODO: move this component to inventory module?
/// Miscellaneous inventory data.
#[derive(Component, Debug)]
pub struct PlayerInventoryState {
    /// The current window ID. Incremented when inventories are opened.
    pub(crate) window_id: u8,
    pub(crate) state_id: Wrapping<i32>,
    /// Tracks what slots have been changed by this client in this tick, so we
    /// don't need to send updates for them.
    pub(crate) slots_changed: u64,
    // TODO: make this a separate modifiable component.
    pub(crate) held_item_slot: u16,
}

impl PlayerInventoryState {
    pub fn held_item_slot(&self) -> u16 {
        self.held_item_slot
    }
}

/// A system for adding [`Despawned`] components to disconnected clients. This
/// works by listening for removed [`Client`] components.
pub fn despawn_disconnected_clients(
    mut commands: Commands,
    mut disconnected_clients: RemovedComponents<Client>,
) {
    for entity in disconnected_clients.iter() {
        if let Some(mut entity) = commands.get_entity(entity) {
            entity.insert(Despawned);
        }
    }
}

pub(crate) fn update_clients() -> SystemConfigs {
    (
        initial_join,
        update_chunk_load_dist,
        respawn,
        send_keepalive,
        read_data_in_view,
        update_view,
        remove_entities,
        teleport,
        update_compass_pos,
        update_tracked_data,
        update_op_level,
        acknowledge_player_actions,
        flush_packets,
    )
        .chain()
}

#[derive(WorldQuery)]
#[world_query(mutable)]
struct ClientJoinQuery {
    entity: Entity,
    client: &'static mut Client,
    loc: &'static Location,
    pos: &'static Position,
    is_hardcore: &'static IsHardcore,
    game_mode: &'static GameMode,
    prev_game_mode: &'static PrevGameMode,
    hashed_seed: &'static HashedSeed,
    view_distance: &'static ViewDistance,
    reduced_debug_info: &'static ReducedDebugInfo,
    has_respawn_screen: &'static HasRespawnScreen,
    is_debug: &'static IsDebug,
    is_flat: &'static IsFlat,
    death_loc: &'static DeathLocation,
}

fn initial_join(
    server: Res<Server>,
    mut clients: Query<ClientJoinQuery, Added<Client>>,
    instances: Query<&Instance>,
    mut commands: Commands,
) {
    for mut q in &mut clients {
        let Ok(instance) = instances.get(q.loc.0) else {
            warn!("Client {:?} joined nonexistent instance {:?}. Disconnecting.", q.entity, q.loc.0);
            commands.entity(q.entity).remove::<Client>();
            continue
        };

        let dimension_names = server
            .dimensions()
            .map(|(_, dim)| dim.name.as_str_ident())
            .collect();

        let dimension_name = server.dimension(instance.dimension()).name.as_str_ident();

        let last_death_location = q.death_loc.0.map(|(id, pos)| GlobalPos {
            dimension_name: server.dimension(id).name.as_str_ident(),
            position: pos,
        });

        // The login packet is prepended so that it's sent before all the other packets.
        // Some packets don't work corectly when sent before the game join packet.
        _ = q.client.enc.prepend_packet(&GameJoinS2c {
            entity_id: 0, // We reserve ID 0 for clients.
            is_hardcore: q.is_hardcore.0,
            game_mode: q.game_mode.clone().into(),
            previous_game_mode: q.prev_game_mode.0.map(|g| g as i8).unwrap_or(-1),
            dimension_names,
            registry_codec: Cow::Borrowed(server.registry_codec()),
            dimension_type_name: dimension_name,
            dimension_name,
            hashed_seed: q.hashed_seed.0 as i64,
            max_players: VarInt(0), // Ignored by clients.
            view_distance: VarInt(q.view_distance.0 as i32),
            simulation_distance: VarInt(16), // TODO.
            reduced_debug_info: q.reduced_debug_info.0,
            enable_respawn_screen: q.has_respawn_screen.0,
            is_debug: q.is_debug.0,
            is_flat: q.is_flat.0,
            last_death_location,
        });

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
            &Location,
            &DeathLocation,
            &HashedSeed,
            &GameMode,
            &PrevGameMode,
            &IsDebug,
            &IsFlat,
        ),
        Changed<Location>,
    >,
    instances: Query<&Instance>,
    server: Res<Server>,
) {
    for (mut client, loc, death_loc, hashed_seed, game_mode, prev_game_mode, is_debug, is_flat) in
        &mut clients
    {
        if client.is_added() {
            // No need to respawn since we are sending the game join packet this tick.
            continue;
        }

        let Ok(instance) = instances.get(loc.0) else {
            warn!("Client respawned in nonexistent instance.");
            continue
        };

        let dimension_name = server.dimension(instance.dimension()).name.as_str_ident();

        let last_death_location = death_loc.0.map(|(id, pos)| GlobalPos {
            dimension_name: server.dimension(id).name.as_str_ident(),
            position: pos,
        });

        client.write_packet(&PlayerRespawnS2c {
            dimension_type_name: dimension_name,
            dimension_name,
            hashed_seed: hashed_seed.0,
            game_mode: (*game_mode).into(),
            previous_game_mode: prev_game_mode.0.map(|g| g as i8).unwrap_or(-1),
            is_debug: is_debug.0,
            is_flat: is_flat.0,
            copy_metadata: true,
            last_death_location,
        });
    }
}

fn update_chunk_load_dist(
    mut clients: Query<(&mut Client, &ViewDistance, &OldViewDistance), Changed<ViewDistance>>,
) {
    for (mut client, dist, old_dist) in &mut clients {
        if client.is_added() {
            // Join game packet includes the view distance.
            continue;
        }

        if dist.0 != old_dist.0 {
            client.write_packet(&ChunkLoadDistanceS2c {
                view_distance: VarInt(dist.0.into()),
            });
        }
    }
}

fn read_data_in_view(
    mut clients: Query<(
        &mut Client,
        &mut EntityRemoveBuffer,
        &OldLocation,
        &OldPosition,
        &OldViewDistance,
    )>,
    instances: Query<&Instance>,
    entities: Query<&McEntity>,
) {
    clients.par_iter_mut().for_each_mut(
        |(client, mut remove_buf, old_loc, old_pos, old_view_dist)| {
            let Ok(instance) = instances.get(old_loc.get()) else {
                return;
            };

            // TODO: remove this and move scratch buffer out of `Client`.
            let client = client.into_inner();

            // Send instance-wide packet data.
            client.write_packet_bytes(&instance.packet_buf);

            // TODO: cache the chunk position?
            let chunk_pos = old_pos.chunk_pos();

            let view = ChunkView::new(chunk_pos, old_view_dist.0);

            // Iterate over all visible chunks from the previous tick.
            view.for_each(|pos| {
                if let Some(cell) = instance.partition.get(&pos) {
                    if cell.chunk_removed && cell.chunk.is_none() {
                        // Chunk was previously loaded and is now deleted.
                        client.write_packet(&UnloadChunkS2c {
                            chunk_x: pos.x,
                            chunk_z: pos.z,
                        });
                    }

                    if let Some(chunk) = &cell.chunk {
                        chunk.mark_viewed();
                    }

                    // Send entity spawn packets for entities entering the client's view.
                    for &(id, src_pos) in &cell.incoming {
                        if src_pos.map_or(true, |p| !view.contains(p)) {
                            // The incoming entity originated from outside the view distance, so it
                            // must be spawned.
                            if let Ok(entity) = entities.get(id) {
                                // Spawn the entity at the old position so that later relative
                                // entity movement packets will not
                                // set the entity to the wrong position.
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
                        if dest_pos.map_or(true, |p| !view.contains(p)) {
                            // The outgoing entity moved outside the view distance, so it must be
                            // despawned.
                            if let Ok(entity) = entities.get(id) {
                                remove_buf.push(entity.protocol_id());
                            }
                        }
                    }

                    // Send all data in the chunk's packet buffer to this client. This will update
                    // entities in the cell, spawn or update the chunk in the cell, or send any
                    // other packet data that was added here by users.
                    client.write_packet_bytes(&cell.packet_buf);
                }
            });
        },
    );
}

/// Updates the clients' view, i.e. the set of chunks that are visible from the
/// client's chunk position.
fn update_view(
    mut clients: Query<
        (
            &mut Client,
            &mut EntityRemoveBuffer,
            &Location,
            &OldLocation,
            &Position,
            &OldPosition,
            &ViewDistance,
            &OldViewDistance,
        ),
        Or<(Changed<Location>, Changed<Position>, Changed<ViewDistance>)>,
    >,
    instances: Query<&Instance>,
    entities: Query<&McEntity>,
) {
    clients.par_iter_mut().for_each_mut(
        |(client, mut remove_buf, loc, old_loc, pos, old_pos, view_dist, old_view_dist)| {
            // TODO: cache chunk pos?
            let view = ChunkView::new(ChunkPos::from_dvec3(pos.0), view_dist.0);
            let old_view = ChunkView::new(ChunkPos::from_dvec3(old_pos.get()), old_view_dist.0);

            let client = client.into_inner();

            // Make sure the center chunk is set before loading chunks! Otherwise the client
            // may ignore the chunk.
            if old_view.pos != view.pos {
                client.write_packet(&ChunkRenderDistanceCenterS2c {
                    chunk_x: VarInt(view.pos.x),
                    chunk_z: VarInt(view.pos.z),
                });
            }

            // Was the client's instance changed?
            if loc.0 != old_loc.get() {
                if let Ok(old_instance) = instances.get(old_loc.get()) {
                    // TODO: only send unload packets when old dimension == new dimension, since the
                    //       client will do the unloading for us in that case?

                    // Unload all chunks and entities in the old view.
                    old_view.for_each(|pos| {
                        if let Some(cell) = old_instance.partition.get(&pos) {
                            // Unload the chunk at this cell if it was loaded.
                            if cell.chunk.is_some() {
                                client.write_packet(&UnloadChunkS2c {
                                    chunk_x: pos.x,
                                    chunk_z: pos.z,
                                });
                            }

                            // Unload all the entities in the cell.
                            for &id in &cell.entities {
                                if let Ok(entity) = entities.get(id) {
                                    remove_buf.push(entity.protocol_id());
                                }
                            }
                        }
                    });
                }

                if let Ok(instance) = instances.get(loc.0) {
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
                } else {
                    warn!("Client entered nonexistent instance ({:?}).", loc.0);
                }
            } else if old_view != view {
                // Client changed their view without changing the instance.

                if let Ok(instance) = instances.get(loc.0) {
                    // Unload chunks and entities in the old view and load chunks and entities in
                    // the new view. We don't need to do any work where the old and new view
                    // overlap.
                    old_view.diff_for_each(view, |pos| {
                        if let Some(cell) = instance.partition.get(&pos) {
                            // Unload the chunk at this cell if it was loaded.
                            if cell.chunk.is_some() {
                                client.write_packet(&UnloadChunkS2c {
                                    chunk_x: pos.x,
                                    chunk_z: pos.z,
                                });
                            }

                            // Unload all the entities in the cell.
                            for &id in &cell.entities {
                                if let Ok(entity) = entities.get(id) {
                                    remove_buf.push(entity.protocol_id());
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
            }
        },
    );
}

fn remove_entities(
    mut clients: Query<(&mut Client, &mut EntityRemoveBuffer), Changed<EntityRemoveBuffer>>,
) {
    for (mut client, mut buf) in &mut clients {
        if !buf.0.is_empty() {
            client.write_packet(&EntitiesDestroyS2c {
                entity_ids: Cow::Borrowed(&buf.0),
            });

            buf.0.clear();
        }
    }
}

fn update_game_mode(mut clients: Query<(&mut Client, &GameMode), Changed<GameMode>>) {
    for (mut client, game_mode) in &mut clients {
        if client.is_added() {
            // Game join packet includes the initial game mode.
            continue;
        }

        client.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::ChangeGameMode,
            value: *game_mode as i32 as f32,
        })
    }
}

fn teleport(
    mut clients: Query<
        (
            &mut Client,
            &mut TeleportState,
            Ref<Position>,
            Ref<Yaw>,
            Ref<Pitch>,
        ),
        Or<(Changed<Position>, Changed<Yaw>, Changed<Pitch>)>,
    >,
) {
    for (mut client, mut state, pos, yaw, pitch) in &mut clients {
        // TODO: skip if client is added?

        let flags = PlayerPositionLookFlags::new()
            .with_x(!pos.is_changed())
            .with_y(!pos.is_changed())
            .with_z(!pos.is_changed())
            .with_y_rot(!yaw.is_changed())
            .with_x_rot(!pitch.is_changed());

        client.write_packet(&PlayerPositionLookS2c {
            position: if pos.is_changed() {
                pos.0.into()
            } else {
                [0.0; 3]
            },
            yaw: if yaw.is_changed() { yaw.0 } else { 0.0 },
            pitch: if pitch.is_changed() { pitch.0 } else { 0.0 },
            flags,
            teleport_id: VarInt(state.teleport_id_counter as i32),
            dismount_vehicle: false, // TODO?
        });

        state.pending_teleports = state.pending_teleports.wrapping_add(1);
        state.teleport_id_counter = state.teleport_id_counter.wrapping_add(1);
    }
}

fn update_compass_pos(mut clients: Query<(&mut Client, &CompassPos), Changed<CompassPos>>) {
    // This also closes the "downloading terrain" screen when first joining, so
    // we should do this after the initial chunks are written.
    for (mut client, compass_pos) in &mut clients {
        client.write_packet(&PlayerSpawnPositionS2c {
            position: compass_pos.0,
            angle: 0.0, // TODO: does this do anything?
        });
    }
}

fn flush_packets(
    mut clients: Query<(Entity, &mut Client), Changed<Client>>,
    mut commands: Commands,
) {
    for (entity, mut client) in &mut clients {
        if let Err(e) = client.flush_packets() {
            warn!("Failed to flush packet queue for client {entity:?}: {e:#}.");
            commands.entity(entity).remove::<Client>();
        }
    }
}

fn update_tracked_data(mut clients: Query<(&mut Client, &McEntity)>) {
    for (mut client, entity) in &mut clients {
        if let TrackedData::Player(player) = &entity.data {
            client.scratch.clear();
            // TODO: should some fields be ignored?
            player.updated_tracked_data(&mut client.scratch);

            if !client.scratch.is_empty() {
                client.scratch.push(0xff);

                let client = client.into_inner();

                client.enc.write_packet(&EntityTrackerUpdateS2c {
                    entity_id: VarInt(0),
                    metadata: client.scratch.as_slice().into(),
                })
            }
        }
    }
}

fn update_op_level(mut clients: Query<(&mut Client, &OpLevel), Changed<OpLevel>>) {
    for (mut client, lvl) in &mut clients {
        client.write_packet(&EntityStatusS2c {
            entity_id: 0,
            entity_status: 24 + lvl.0,
        });
    }
}

fn acknowledge_player_actions(
    mut clients: Query<(&mut Client, &mut PlayerActionSequence), Changed<PlayerActionSequence>>,
) {
    for (mut client, mut action_seq) in &mut clients {
        if action_seq.0 != 0 {
            client.write_packet(&PlayerActionResponseS2c {
                sequence: VarInt(action_seq.0),
            });

            action_seq.0 = 0;
        }
    }
}

fn send_keepalive(
    mut clients: Query<(Entity, &mut Client, &mut KeepaliveState)>,
    server: Res<Server>,
    mut commands: Commands,
) {
    if server.current_tick() % (server.tps() * 10) == 0 {
        let mut rng = rand::thread_rng();

        for (entity, mut client, mut state) in &mut clients {
            if state.got_keepalive {
                let id = rng.gen();
                client.write_packet(&KeepAliveS2c { id });

                state.got_keepalive = false;
                state.last_keepalive_id = id;
                state.keepalive_sent_time = Instant::now();
            } else {
                warn!("Client {entity:?} timed out (no keepalive response)");
                commands.entity(entity).remove::<Client>();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use bevy_app::App;
    use bevy_ecs::world::EntityMut;
    use valence_protocol::packet::s2c::play::ChunkDataS2c;
    use valence_protocol::packet::S2cPlayPacket;

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

        let mut client = app.world.entity_mut(client_ent);

        client.get_mut::<Position>().unwrap().set([8.0, 0.0, 8.0]);
        client.get_mut::<ViewDistance>().unwrap().0 = 6;

        // Tick
        app.update();
        let mut client = app.world.entity_mut(client_ent);

        let mut loaded_chunks = BTreeSet::new();

        for pkt in client_helper.collect_sent().unwrap() {
            if let S2cPlayPacket::ChunkDataS2c(ChunkDataS2c {
                chunk_x, chunk_z, ..
            }) = pkt
            {
                assert!(
                    loaded_chunks.insert(ChunkPos::new(chunk_x, chunk_z)),
                    "({chunk_x}, {chunk_z})"
                );
            }
        }

        for pos in view(&client).iter() {
            assert!(loaded_chunks.contains(&pos), "{pos:?}");
        }

        assert!(!loaded_chunks.is_empty());

        // Move the client to the adjacent chunk.
        client.get_mut::<Position>().unwrap().set([24.0, 0.0, 24.0]);

        // Tick
        app.update();
        let client = app.world.entity_mut(client_ent);

        for pkt in client_helper.collect_sent().unwrap() {
            match pkt {
                S2cPlayPacket::ChunkDataS2c(ChunkDataS2c {
                    chunk_x, chunk_z, ..
                }) => {
                    assert!(
                        loaded_chunks.insert(ChunkPos::new(chunk_x, chunk_z)),
                        "({chunk_x}, {chunk_z})"
                    );
                }
                S2cPlayPacket::UnloadChunkS2c(UnloadChunkS2c { chunk_x, chunk_z }) => {
                    assert!(
                        loaded_chunks.remove(&ChunkPos::new(chunk_x, chunk_z)),
                        "({chunk_x}, {chunk_z})"
                    );
                }
                _ => {}
            }
        }

        for pos in view(&client).iter() {
            assert!(loaded_chunks.contains(&pos), "{pos:?}");
        }
    }

    fn view(client: &EntityMut) -> ChunkView {
        let chunk_pos = client.get::<Position>().unwrap().chunk_pos();
        let view_dist = client.get::<ViewDistance>().unwrap().0;

        ChunkView::new(chunk_pos, view_dist)
    }
}
