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

use std::borrow::Cow;
use std::collections::BTreeSet;
use std::fmt;
use std::net::IpAddr;
use std::ops::Deref;
use std::time::Instant;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::query::WorldQuery;
use bevy_ecs::system::Command;
use byteorder::{NativeEndian, ReadBytesExt};
use bytes::{Bytes, BytesMut};
use glam::{DVec3, Vec3};
use packet::{
    DeathMessageS2c, DisconnectS2c, GameEventKind, GameJoinS2c, GameStateChangeS2c,
    PlayerRespawnS2c, PlayerSpawnPositionS2c,
};
use tracing::warn;
use uuid::Uuid;
use valence_biome::BiomeRegistry;
use valence_core::block_pos::BlockPos;
use valence_core::chunk_pos::{ChunkPos, ChunkView};
use valence_core::despawn::Despawned;
use valence_core::game_mode::GameMode;
use valence_core::ident::Ident;
use valence_core::particle::{Particle, ParticleS2c};
use valence_core::property::Property;
use valence_core::protocol::encode::{PacketEncoder, WritePacket};
use valence_core::protocol::global_pos::GlobalPos;
use valence_core::protocol::packet::sound::{PlaySoundS2c, Sound, SoundCategory};
use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::{Encode, Packet};
use valence_core::text::{IntoText, Text};
use valence_core::uuid::UniqueId;
use valence_entity::packet::{
    EntitiesDestroyS2c, EntityStatusS2c, EntityTrackerUpdateS2c, EntityVelocityUpdateS2c,
};
use valence_entity::player::PlayerEntityBundle;
use valence_entity::query::EntityInitQuery;
use valence_entity::tracked_data::TrackedData;
use valence_entity::{
    ClearEntityChangesSet, EntityId, EntityLayerId, EntityStatus, Look, OldPosition, Position,
    Velocity,
};
use valence_layer::packet::{
    ChunkBiome, ChunkBiomeDataS2c, ChunkLoadDistanceS2c, ChunkRenderDistanceCenterS2c,
    UnloadChunkS2c,
};
use valence_layer::{ChunkLayer, EntityLayer, UpdateLayersPostClientSet, UpdateLayersPreClientSet};
use valence_registry::RegistrySet;

pub mod action;
pub mod command;
pub mod custom_payload;
pub mod event_loop;
pub mod hand_swing;
pub mod interact_block;
pub mod interact_entity;
pub mod interact_item;
pub mod keepalive;
pub mod message;
pub mod movement;
pub mod op_level;
pub mod packet;
pub mod resource_pack;
pub mod settings;
pub mod spawn;
pub mod status;
pub mod teleport;
pub mod title;
pub mod weather;

pub struct ClientPlugin;

/// The [`SystemSet`] in [`PostUpdate`] where clients have their packet buffer
/// flushed. Any system that writes packets to clients should happen _before_
/// this. Otherwise, the data will arrive one tick late.
#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct FlushPacketsSet;

/// The [`SystemSet`] in [`PreUpdate`] where new clients should be
/// spawned. Systems that need to perform initialization work on clients before
/// users get access to it should run _after_ this set.
#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct SpawnClientsSet;

/// The system set where various facets of the client are updated. Systems that
/// modify chunks should run _before_ this.
#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct UpdateClientsSet;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                (
                    spawn::initial_join.after(RegistrySet),
                    update_chunk_load_dist,
                    handle_layer_messages.after(update_chunk_load_dist),
                    update_view_and_layers
                        .after(spawn::initial_join)
                        .after(handle_layer_messages),
                    cleanup_chunks_after_client_despawn.after(update_view_and_layers),
                    spawn::update_respawn_position.after(update_view_and_layers),
                    spawn::respawn.after(spawn::update_respawn_position),
                    remove_entities.after(update_view_and_layers),
                    update_old_view_dist.after(update_view_and_layers),
                    update_game_mode,
                    update_tracked_data,
                    init_tracked_data,
                )
                    .in_set(UpdateClientsSet),
                flush_packets.in_set(FlushPacketsSet),
            ),
        )
        .configure_set(PreUpdate, SpawnClientsSet)
        .configure_sets(
            PostUpdate,
            (
                UpdateClientsSet
                    .after(UpdateLayersPreClientSet)
                    .before(UpdateLayersPostClientSet)
                    .before(FlushPacketsSet),
                ClearEntityChangesSet.after(UpdateClientsSet),
                FlushPacketsSet,
                UpdateLayersPostClientSet.after(FlushPacketsSet),
            ),
        );

        event_loop::build(app);
        movement::build(app);
        command::build(app);
        keepalive::build(app);
        interact_entity::build(app);
        settings::build(app);
        action::build(app);
        teleport::build(app);
        // weather::build(app);
        message::build(app);
        custom_payload::build(app);
        hand_swing::build(app);
        interact_block::build(app);
        interact_item::build(app);
        op_level::build(app);
        resource_pack::build(app);
        status::build(app);
    }
}

/// The bundle of components needed for clients to function. All components are
/// required unless otherwise stated.
#[derive(Bundle)]
pub struct ClientBundle {
    pub marker: ClientMarker,
    pub client: Client,
    pub settings: settings::ClientSettings,
    pub entity_remove_buf: EntityRemoveBuf,
    pub username: Username,
    pub ip: Ip,
    pub properties: Properties,
    pub respawn_pos: RespawnPosition,
    pub op_level: op_level::OpLevel,
    pub action_sequence: action::ActionSequence,
    pub view_distance: ViewDistance,
    pub old_view_distance: OldViewDistance,
    pub visible_chunk_layer: VisibleChunkLayer,
    pub old_visible_chunk_layer: OldVisibleChunkLayer,
    pub visible_entity_layers: VisibleEntityLayers,
    pub old_visible_entity_layers: OldVisibleEntityLayers,
    pub keepalive_state: keepalive::KeepaliveState,
    pub ping: Ping,
    pub teleport_state: teleport::TeleportState,
    pub game_mode: GameMode,
    pub prev_game_mode: spawn::PrevGameMode,
    pub death_location: spawn::DeathLocation,
    pub is_hardcore: spawn::IsHardcore,
    pub hashed_seed: spawn::HashedSeed,
    pub reduced_debug_info: spawn::ReducedDebugInfo,
    pub has_respawn_screen: spawn::HasRespawnScreen,
    pub is_debug: spawn::IsDebug,
    pub is_flat: spawn::IsFlat,
    pub portal_cooldown: spawn::PortalCooldown,
    pub player: PlayerEntityBundle,
}

impl ClientBundle {
    pub fn new(args: ClientBundleArgs) -> Self {
        Self {
            marker: ClientMarker,
            client: Client {
                conn: args.conn,
                enc: args.enc,
            },
            settings: settings::ClientSettings::default(),
            entity_remove_buf: EntityRemoveBuf(vec![]),
            username: Username(args.username),
            ip: Ip(args.ip),
            properties: Properties(args.properties),
            respawn_pos: RespawnPosition::default(),
            op_level: op_level::OpLevel::default(),
            action_sequence: action::ActionSequence::default(),
            view_distance: ViewDistance::default(),
            old_view_distance: OldViewDistance(2),
            visible_chunk_layer: VisibleChunkLayer::default(),
            old_visible_chunk_layer: OldVisibleChunkLayer(Entity::PLACEHOLDER),
            visible_entity_layers: VisibleEntityLayers::default(),
            old_visible_entity_layers: OldVisibleEntityLayers(BTreeSet::new()),
            keepalive_state: keepalive::KeepaliveState::new(),
            ping: Ping::default(),
            teleport_state: teleport::TeleportState::new(),
            game_mode: GameMode::default(),
            prev_game_mode: spawn::PrevGameMode::default(),
            death_location: spawn::DeathLocation::default(),
            is_hardcore: spawn::IsHardcore::default(),
            is_flat: spawn::IsFlat::default(),
            has_respawn_screen: spawn::HasRespawnScreen::default(),
            hashed_seed: spawn::HashedSeed::default(),
            reduced_debug_info: spawn::ReducedDebugInfo::default(),
            is_debug: spawn::IsDebug::default(),
            portal_cooldown: spawn::PortalCooldown::default(),
            player: PlayerEntityBundle {
                uuid: UniqueId(args.uuid),
                ..Default::default()
            },
        }
    }
}

/// Arguments for [`ClientBundle::new`].
pub struct ClientBundleArgs {
    /// The username for the client.
    pub username: String,
    pub uuid: Uuid,
    pub ip: IpAddr,
    pub properties: Vec<Property>,
    pub conn: Box<dyn ClientConnection>,
    /// The packet encoder to use. This should be in sync with [`Self::conn`].
    pub enc: PacketEncoder,
}

/// Marker [`Component`] for client entities. This component should exist even
/// if the client is disconnected.
#[derive(Component, Copy, Clone)]
pub struct ClientMarker;

/// The main client component. Contains the underlying network connection and
/// packet buffer.
///
/// The component is removed when the client is disconnected. You are allowed to
/// remove the component yourself.
#[derive(Component)]
pub struct Client {
    conn: Box<dyn ClientConnection>,
    enc: PacketEncoder,
}

/// Represents the bidirectional packet channel between the server and a client
/// in the "play" state.
pub trait ClientConnection: Send + Sync + 'static {
    /// Sends encoded clientbound packet data. This function must not block and
    /// the data should be sent as soon as possible.
    fn try_send(&mut self, bytes: BytesMut) -> anyhow::Result<()>;
    /// Receives the next pending serverbound packet. This must return
    /// immediately without blocking.
    fn try_recv(&mut self) -> anyhow::Result<Option<ReceivedPacket>>;
    /// The number of pending packets waiting to be received via
    /// [`Self::try_recv`].
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Clone, Debug)]
pub struct ReceivedPacket {
    /// The moment in time this packet arrived. This is _not_ the instant this
    /// packet was returned from [`ClientConnection::try_recv`].
    pub timestamp: Instant,
    /// This packet's ID.
    pub id: i32,
    /// The content of the packet, excluding the leading varint packet ID.
    pub body: Bytes,
}

impl Drop for Client {
    fn drop(&mut self) {
        _ = self.flush_packets();
    }
}

/// Writes packets into this client's packet buffer. The buffer is flushed at
/// the end of the tick.
impl WritePacket for Client {
    fn write_packet_fallible<P>(&mut self, packet: &P) -> anyhow::Result<()>
    where
        P: Packet + Encode,
    {
        self.enc.write_packet_fallible(packet)
    }

    fn write_packet_bytes(&mut self, bytes: &[u8]) {
        self.enc.write_packet_bytes(bytes)
    }
}

impl Client {
    pub fn connection(&self) -> &dyn ClientConnection {
        self.conn.as_ref()
    }

    pub fn connection_mut(&mut self) -> &mut dyn ClientConnection {
        self.conn.as_mut()
    }

    /// Flushes the packet queue to the underlying connection.
    ///
    /// This is called automatically at the end of the tick and when the client
    /// is dropped. Unless you're in a hurry, there's usually no reason to
    /// call this method yourself.
    ///
    /// Returns an error if flushing was unsuccessful.
    pub fn flush_packets(&mut self) -> anyhow::Result<()> {
        let bytes = self.enc.take();
        if !bytes.is_empty() {
            self.conn.try_send(bytes)
        } else {
            Ok(())
        }
    }

    /// Kills the client and shows `message` on the death screen. If an entity
    /// killed the player, you should supply it as `killer`.
    pub fn kill<'a>(&mut self, message: impl IntoText<'a>) {
        self.write_packet(&DeathMessageS2c {
            player_id: VarInt(0),
            message: message.into_cow_text(),
        });
    }

    /// Respawns client. Optionally can roll the credits before respawning.
    pub fn win_game(&mut self, show_credits: bool) {
        self.write_packet(&GameStateChangeS2c {
            kind: GameEventKind::WinGame,
            value: if show_credits { 1.0 } else { 0.0 },
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
            position: position.into(),
            offset: offset.into(),
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
            position: (position * 8.0).as_ivec3(),
            volume,
            pitch,
            seed: rand::random(),
        });
    }

    /// `velocity` is in m/s.
    pub fn set_velocity(&mut self, velocity: impl Into<Vec3>) {
        self.write_packet(&EntityVelocityUpdateS2c {
            entity_id: VarInt(0),
            velocity: Velocity(velocity.into()).to_packet_units(),
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
    fn apply(self, world: &mut World) {
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
pub struct EntityRemoveBuf(Vec<VarInt>);

impl EntityRemoveBuf {
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

#[derive(Component, Clone, PartialEq, Eq, Default, Debug)]
pub struct Username(pub String);

impl Username {
    pub fn is_valid(&self) -> bool {
        is_valid_username(&self.0)
    }
}

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Returns whether or not the given string is a valid Minecraft username.
///
/// A valid username is 3 to 16 characters long with only ASCII alphanumeric
/// characters. The username must match the regex `^[a-zA-Z0-9_]{3,16}$` to be
/// considered valid.
///
/// # Examples
///
/// ```
/// # use valence_client::is_valid_username;
///
/// assert!(is_valid_username("00a"));
/// assert!(is_valid_username("jeb_"));
///
/// assert!(!is_valid_username("notavalidusername"));
/// assert!(!is_valid_username("NotValid!"));
/// ```
pub fn is_valid_username(username: &str) -> bool {
    (3..=16).contains(&username.len())
        && username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[derive(Component, Clone, PartialEq, Eq, Default, Debug)]
pub struct Properties(pub Vec<Property>);

impl Properties {
    /// Finds the property with the name "textures".
    pub fn textures(&self) -> Option<&Property> {
        self.0.iter().find(|prop| prop.name == "textures")
    }

    /// Finds the property with the name "textures".
    pub fn textures_mut(&mut self) -> Option<&mut Property> {
        self.0.iter_mut().find(|prop| prop.name == "textures")
    }
}

impl From<Vec<Property>> for Properties {
    fn from(value: Vec<Property>) -> Self {
        Self(value)
    }
}

impl Deref for Properties {
    type Target = [Property];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Component, Clone, PartialEq, Eq, Debug)]
pub struct Ip(pub IpAddr);

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

#[derive(Component, Clone, PartialEq, Eq, Debug)]
pub struct ViewDistance(u8);

impl ViewDistance {
    pub fn new(dist: u8) -> Self {
        let mut new = Self(0);
        new.set(dist);
        new
    }

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

#[derive(WorldQuery, Copy, Clone, Debug)]
pub struct View {
    pub pos: &'static Position,
    pub view_dist: &'static ViewDistance,
}

impl ViewItem<'_> {
    pub fn get(&self) -> ChunkView {
        ChunkView::new(self.pos.chunk_pos(), self.view_dist.0)
    }
}

#[derive(WorldQuery, Copy, Clone, Debug)]
pub struct OldView {
    pub old_pos: &'static OldPosition,
    pub old_view_dist: &'static OldViewDistance,
}

impl OldViewItem<'_> {
    pub fn get(&self) -> ChunkView {
        ChunkView::new(self.old_pos.chunk_pos(), self.old_view_dist.0)
    }
}

/// Delay measured in milliseconds. Negative values indicate absence.
#[derive(Component, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Ping(pub i32);

impl Default for Ping {
    fn default() -> Self {
        Self(-1)
    }
}

#[derive(Component, Copy, Clone, PartialEq, Eq, Debug)]
pub struct VisibleChunkLayer(pub Entity);

impl Default for VisibleChunkLayer {
    fn default() -> Self {
        Self(Entity::PLACEHOLDER)
    }
}

#[derive(Component, PartialEq, Eq, Debug)]
pub struct OldVisibleChunkLayer(Entity);

impl OldVisibleChunkLayer {
    pub fn get(&self) -> Entity {
        self.0
    }
}

#[derive(Component, Default, Debug)]
pub struct VisibleEntityLayers(pub BTreeSet<Entity>);

#[derive(Component, Default, Debug)]
pub struct OldVisibleEntityLayers(BTreeSet<Entity>);

impl OldVisibleEntityLayers {
    pub fn get(&self) -> &BTreeSet<Entity> {
        &self.0
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

fn update_chunk_load_dist(
    mut clients: Query<(&mut Client, &ViewDistance, &OldViewDistance), Changed<ViewDistance>>,
) {
    for (mut client, dist, old_dist) in &mut clients {
        if client.is_added() {
            // Join game packet includes the view distance.
            continue;
        }

        if dist.0 != old_dist.0 {
            // Note: This packet is just aesthetic.
            client.write_packet(&ChunkLoadDistanceS2c {
                view_distance: VarInt(dist.0.into()),
            });
        }
    }
}

fn handle_layer_messages(
    mut clients: Query<(
        Entity,
        &EntityId,
        &mut Client,
        &mut EntityRemoveBuf,
        OldView,
        &OldVisibleChunkLayer,
        &mut VisibleEntityLayers,
        &OldVisibleEntityLayers,
    )>,
    chunk_layers: Query<&ChunkLayer>,
    entity_layers: Query<&EntityLayer>,
    entities: Query<(EntityInitQuery, &OldPosition)>,
) {
    clients.par_iter_mut().for_each_mut(
        |(
            self_entity,
            self_entity_id,
            mut client,
            mut remove_buf,
            old_view,
            old_visible_chunk_layer,
            mut visible_entity_layers,
            old_visible_entity_layers,
        )| {
            let old_view = old_view.get();

            // Chunk layer messages
            if let Ok(chunk_layer) = chunk_layers.get(old_visible_chunk_layer.get()) {
                let messages = chunk_layer.messages();
                let bytes = messages.bytes();

                // Global messages
                for (msg, range) in messages.iter_global() {
                    match msg {
                        valence_layer::chunk::GlobalMsg::Packet => {
                            client.write_packet_bytes(&bytes[range]);
                        }
                        valence_layer::chunk::GlobalMsg::PacketExcept { except } => {
                            if self_entity != except {
                                client.write_packet_bytes(&bytes[range]);
                            }
                        }
                    }
                }

                let mut chunk_biome_buf = vec![];

                // Local messages
                messages.query_local(old_view, |msg, range| match msg {
                    valence_layer::chunk::LocalMsg::PacketAt { .. } => {
                        client.write_packet_bytes(&bytes[range]);
                    }
                    valence_layer::chunk::LocalMsg::ChangeBiome { pos } => {
                        chunk_biome_buf.push(ChunkBiome {
                            pos,
                            data: &bytes[range],
                        });
                    }
                    valence_layer::chunk::LocalMsg::ChangeChunkState { pos } => {
                        match &bytes[range] {
                            [ChunkLayer::LOAD, .., ChunkLayer::UNLOAD] => {
                                // Chunk is being loaded and unloaded on the
                                // same tick, so there's no need to do anything.
                            }
                            [.., ChunkLayer::LOAD | ChunkLayer::OVERWRITE] => {
                                // Load chunk.
                                if let Some(chunk) = chunk_layer.chunk(pos) {
                                    chunk.write_init_packets(&mut *client, pos, chunk_layer.info());
                                    chunk.inc_viewer_count();
                                }
                            }
                            [.., ChunkLayer::UNLOAD] => {
                                // Unload chunk.
                                client.write_packet(&UnloadChunkS2c { pos });
                            }
                            _ => unreachable!("invalid message data while changing chunk state"),
                        }
                    }
                });

                if !chunk_biome_buf.is_empty() {
                    client.write_packet(&ChunkBiomeDataS2c {
                        chunks: chunk_biome_buf.into(),
                    });
                }
            }

            // Entity layer messages
            for &layer_id in &old_visible_entity_layers.0 {
                if let Ok(layer) = entity_layers.get(layer_id) {
                    let messages = layer.messages();
                    let bytes = messages.bytes();

                    // Global messages
                    for (msg, range) in messages.iter_global() {
                        match msg {
                            valence_layer::entity::GlobalMsg::Packet => {
                                client.write_packet_bytes(&bytes[range]);
                            }
                            valence_layer::entity::GlobalMsg::PacketExcept { except } => {
                                if self_entity != except {
                                    client.write_packet_bytes(&bytes[range]);
                                }
                            }
                            valence_layer::entity::GlobalMsg::DespawnLayer => {
                                // Remove this entity layer. The changes to the visible entity layer
                                // set will be detected by the `update_view_and_layers` system and
                                // despawning of entities will happen there.
                                visible_entity_layers.0.remove(&layer_id);
                            }
                        }
                    }

                    // Local messages
                    messages.query_local(old_view, |msg, range| match msg {
                        valence_layer::entity::LocalMsg::PacketAt { pos: _ } => {
                            client.write_packet_bytes(&bytes[range]);
                        }
                        valence_layer::entity::LocalMsg::PacketAtExcept { pos: _, except } => {
                            if self_entity != except {
                                client.write_packet_bytes(&bytes[range]);
                            }
                        }
                        valence_layer::entity::LocalMsg::SpawnEntity { pos: _, src_layer } => {
                            if !old_visible_entity_layers.0.contains(&src_layer) {
                                let mut bytes = &bytes[range];

                                while let Ok(u64) = bytes.read_u64::<NativeEndian>() {
                                    let entity = Entity::from_bits(u64);

                                    if self_entity != entity {
                                        if let Ok((init, old_pos)) = entities.get(entity) {
                                            // Spawn at the entity's old position since we may get a
                                            // relative movement packet for this entity in a later
                                            // iteration of the loop.
                                            init.write_init_packets(old_pos.get(), &mut *client);
                                        }
                                    }
                                }
                            }
                        }
                        valence_layer::entity::LocalMsg::SpawnEntityTransition {
                            pos: _,
                            src_pos,
                        } => {
                            if !old_view.contains(src_pos) {
                                let mut bytes = &bytes[range];

                                while let Ok(u64) = bytes.read_u64::<NativeEndian>() {
                                    let entity = Entity::from_bits(u64);

                                    if self_entity != entity {
                                        if let Ok((init, old_pos)) = entities.get(entity) {
                                            // Spawn at the entity's old position since we may get a
                                            // relative movement packet for this entity in a later
                                            // iteration of the loop.
                                            init.write_init_packets(old_pos.get(), &mut *client);
                                        }
                                    }
                                }
                            }
                        }
                        valence_layer::entity::LocalMsg::DespawnEntity { pos: _, dest_layer } => {
                            if !old_visible_entity_layers.0.contains(&dest_layer) {
                                let mut bytes = &bytes[range];

                                while let Ok(id) = bytes.read_i32::<NativeEndian>() {
                                    if self_entity_id.get() != id {
                                        remove_buf.push(id);
                                    }
                                }
                            }
                        }
                        valence_layer::entity::LocalMsg::DespawnEntityTransition {
                            pos: _,
                            dest_pos,
                        } => {
                            if !old_view.contains(dest_pos) {
                                let mut bytes = &bytes[range];

                                while let Ok(id) = bytes.read_i32::<NativeEndian>() {
                                    if self_entity_id.get() != id {
                                        remove_buf.push(id);
                                    }
                                }
                            }
                        }
                    });
                }
            }
        },
    );
}

fn update_view_and_layers(
    mut clients: Query<
        (
            Entity,
            &mut Client,
            &mut EntityRemoveBuf,
            &VisibleChunkLayer,
            &mut OldVisibleChunkLayer,
            Ref<VisibleEntityLayers>,
            &mut OldVisibleEntityLayers,
            &Position,
            &OldPosition,
            &ViewDistance,
            &OldViewDistance,
        ),
        Or<(
            Changed<VisibleChunkLayer>,
            Changed<VisibleEntityLayers>,
            Changed<Position>,
            Changed<ViewDistance>,
        )>,
    >,
    chunk_layers: Query<&ChunkLayer>,
    entity_layers: Query<&EntityLayer>,
    entity_ids: Query<&EntityId>,
    entity_init: Query<(EntityInitQuery, &Position)>,
) {
    clients.par_iter_mut().for_each_mut(
        |(
            self_entity,
            mut client,
            mut remove_buf,
            chunk_layer,
            mut old_chunk_layer,
            visible_entity_layers,
            mut old_visible_entity_layers,
            pos,
            old_pos,
            view_dist,
            old_view_dist,
        )| {
            let view = ChunkView::new(ChunkPos::from_dvec3(pos.0), view_dist.0);
            let old_view = ChunkView::new(ChunkPos::from_dvec3(old_pos.get()), old_view_dist.0);

            // Make sure the center chunk is set before loading chunks! Otherwise the client
            // may ignore the chunk.
            if old_view.pos != view.pos {
                client.write_packet(&ChunkRenderDistanceCenterS2c {
                    chunk_x: VarInt(view.pos.x),
                    chunk_z: VarInt(view.pos.z),
                });
            }

            // Was the client's chunk layer changed?
            if old_chunk_layer.0 != chunk_layer.0 {
                // Unload all chunks in the old view.
                // TODO: can we skip this step if old dimension != new dimension?
                if let Ok(layer) = chunk_layers.get(old_chunk_layer.0) {
                    for pos in old_view.iter() {
                        if let Some(chunk) = layer.chunk(pos) {
                            client.write_packet(&UnloadChunkS2c { pos });
                            chunk.dec_viewer_count();
                        }
                    }
                }

                // Load all chunks in the new view.
                if let Ok(layer) = chunk_layers.get(chunk_layer.0) {
                    for pos in view.iter() {
                        if let Some(chunk) = layer.chunk(pos) {
                            chunk.write_init_packets(&mut *client, pos, layer.info());
                            chunk.inc_viewer_count();
                        }
                    }
                }

                // Unload all entities from the old view in all old visible entity layers.
                // TODO: can we skip this step if old dimension != new dimension?
                for &layer in &old_visible_entity_layers.0 {
                    if let Ok(layer) = entity_layers.get(layer) {
                        for pos in old_view.iter() {
                            for entity in layer.entities_at(pos) {
                                if self_entity != entity {
                                    if let Ok(id) = entity_ids.get(entity) {
                                        remove_buf.push(id.get());
                                    }
                                }
                            }
                        }
                    }
                }

                // Load all entities in the new view from all new visible entity layers.
                for &layer in &visible_entity_layers.0 {
                    if let Ok(layer) = entity_layers.get(layer) {
                        for pos in view.iter() {
                            for entity in layer.entities_at(pos) {
                                if self_entity != entity {
                                    if let Ok((init, pos)) = entity_init.get(entity) {
                                        init.write_init_packets(pos.get(), &mut *client);
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                // Update the client's visible entity layers.
                if visible_entity_layers.is_changed() {
                    // Unload all entity layers that are no longer visible in the old view.
                    for &layer in old_visible_entity_layers
                        .0
                        .difference(&visible_entity_layers.0)
                    {
                        if let Ok(layer) = entity_layers.get(layer) {
                            for pos in old_view.iter() {
                                for entity in layer.entities_at(pos) {
                                    if self_entity != entity {
                                        if let Ok(id) = entity_ids.get(entity) {
                                            remove_buf.push(id.get());
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Load all entity layers that are newly visible in the old view.
                    for &layer in visible_entity_layers
                        .0
                        .difference(&old_visible_entity_layers.0)
                    {
                        if let Ok(layer) = entity_layers.get(layer) {
                            for pos in old_view.iter() {
                                for entity in layer.entities_at(pos) {
                                    if self_entity != entity {
                                        if let Ok((init, pos)) = entity_init.get(entity) {
                                            init.write_init_packets(pos.get(), &mut *client);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Update the client's view (chunk position and view distance)
                if old_view != view {
                    // Unload chunks and entities in the old view and load chunks and entities in
                    // the new view. We don't need to do any work where the old and new view
                    // overlap.

                    // Unload chunks in the old view.
                    if let Ok(layer) = chunk_layers.get(chunk_layer.0) {
                        for pos in old_view.diff(view) {
                            if let Some(chunk) = layer.chunk(pos) {
                                client.write_packet(&UnloadChunkS2c { pos });
                                chunk.dec_viewer_count();
                            }
                        }
                    }

                    // Load chunks in the new view.
                    if let Ok(layer) = chunk_layers.get(chunk_layer.0) {
                        for pos in view.diff(old_view) {
                            if let Some(chunk) = layer.chunk(pos) {
                                chunk.write_init_packets(&mut *client, pos, layer.info());
                                chunk.inc_viewer_count();
                            }
                        }
                    }

                    // Unload entities from the new visible layers (since we updated it above).
                    for &layer in &visible_entity_layers.0 {
                        if let Ok(layer) = entity_layers.get(layer) {
                            for pos in old_view.diff(view) {
                                for entity in layer.entities_at(pos) {
                                    if self_entity != entity {
                                        if let Ok(id) = entity_ids.get(entity) {
                                            remove_buf.push(id.get());
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Load entities from the new visible layers.
                    for &layer in &visible_entity_layers.0 {
                        if let Ok(layer) = entity_layers.get(layer) {
                            for pos in view.diff(old_view) {
                                for entity in layer.entities_at(pos) {
                                    if self_entity != entity {
                                        if let Ok((init, pos)) = entity_init.get(entity) {
                                            init.write_init_packets(pos.get(), &mut *client);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Update the old layers.

            old_chunk_layer.0 = chunk_layer.0;

            if visible_entity_layers.is_changed() {
                old_visible_entity_layers
                    .0
                    .clone_from(&visible_entity_layers.0);
            }
        },
    );
}

/// Removes all the entities that are queued to be removed for each client.
fn remove_entities(
    mut clients: Query<(&mut Client, &mut EntityRemoveBuf), Changed<EntityRemoveBuf>>,
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

fn update_old_view_dist(
    mut clients: Query<(&mut OldViewDistance, &ViewDistance), Changed<ViewDistance>>,
) {
    for (mut old_dist, dist) in &mut clients {
        old_dist.0 = dist.0;
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

fn init_tracked_data(mut clients: Query<(&mut Client, &TrackedData), Added<TrackedData>>) {
    for (mut client, tracked_data) in &mut clients {
        if let Some(init_data) = tracked_data.init_data() {
            client.write_packet(&EntityTrackerUpdateS2c {
                entity_id: VarInt(0),
                metadata: init_data.into(),
            });
        }
    }
}

fn update_tracked_data(mut clients: Query<(&mut Client, &TrackedData)>) {
    for (mut client, tracked_data) in &mut clients {
        if let Some(update_data) = tracked_data.update_data() {
            client.write_packet(&EntityTrackerUpdateS2c {
                entity_id: VarInt(0),
                metadata: update_data.into(),
            });
        }
    }
}

/// Decrement viewer count of chunks when the client is despawned.
fn cleanup_chunks_after_client_despawn(
    mut clients: Query<(View, &VisibleChunkLayer), (With<ClientMarker>, With<Despawned>)>,
    chunk_layers: Query<&ChunkLayer>,
) {
    for (view, layer) in &mut clients {
        if let Ok(layer) = chunk_layers.get(layer.0) {
            for pos in view.get().iter() {
                if let Some(chunk) = layer.chunk(pos) {
                    chunk.dec_viewer_count();
                }
            }
        }
    }
}
