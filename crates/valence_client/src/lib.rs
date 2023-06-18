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

use std::borrow::Cow;
use std::fmt;
use std::net::IpAddr;
use std::ops::Deref;
use std::time::Instant;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::query::WorldQuery;
use bevy_ecs::system::Command;
use bytes::{Bytes, BytesMut};
use glam::{DVec3, Vec3};
use layer::ClientLayerSet;
use packet::{
    DeathMessageS2c, DisconnectS2c, GameEventKind, GameJoinS2c, GameStateChangeS2c,
    PlayerRespawnS2c, PlayerSpawnPositionS2c, PlayerSpawnS2c,
};
use rand::Rng;
use tracing::{info, warn};
use uuid::Uuid;
use valence_biome::BiomeRegistry;
use valence_core::block_pos::BlockPos;
use valence_core::chunk_pos::{ChunkPos, ChunkView};
use valence_core::despawn::Despawned;
use valence_core::game_mode::GameMode;
use valence_core::ident::Ident;
use valence_core::layer::Layer;
use valence_core::particle::{Particle, ParticleS2c};
use valence_core::property::Property;
use valence_core::protocol::byte_angle::ByteAngle;
use valence_core::protocol::encode::{PacketEncoder, WritePacket};
use valence_core::protocol::global_pos::GlobalPos;
use valence_core::protocol::packet::sound::{PlaySoundS2c, Sound, SoundCategory};
use valence_core::protocol::var_int::VarInt;
use valence_core::protocol::{Encode, Packet};
use valence_core::scratch::ScratchBuf;
use valence_core::text::Text;
use valence_core::uuid::UniqueId;
use valence_core::Server;
use valence_entity::packet::{
    EntitiesDestroyS2c, EntitySetHeadYawS2c, EntitySpawnS2c, EntityStatusS2c,
    EntityTrackerUpdateS2c, EntityVelocityUpdateS2c, ExperienceOrbSpawnS2c,
};
use valence_entity::player::PlayerEntityBundle;
use valence_entity::{
    ClearEntityChangesSet, EntityId, EntityKind, EntityStatus, HeadYaw, Location, Look, ObjectData,
    OldLocation, OldPosition, OnGround, PacketByteRange, Position, TrackedData, Velocity,
};
use valence_instance::packet::{
    ChunkLoadDistanceS2c, ChunkRenderDistanceCenterS2c, UnloadChunkS2c,
};
use valence_instance::{ClearInstanceChangesSet, Instance, WriteUpdatePacketsToInstancesSet};
use valence_registry::codec::RegistryCodec;
use valence_registry::tags::TagsRegistry;
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
pub mod layer;
pub mod message;
pub mod movement;
pub mod op_level;
pub mod packet;
pub mod resource_pack;
pub mod settings;
pub mod status;
pub mod teleport;
pub mod title;
pub mod weather;

pub struct ClientPlugin;

/// When clients have their packet buffer flushed. Any system that writes
/// packets to clients should happen _before_ this. Otherwise, the data
/// will arrive one tick late.
#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct FlushPacketsSet;

/// The [`SystemSet`] in [`CoreSet::PreUpdate`] where new clients should be
/// spawned. Systems that need to perform initialization work on clients before
/// users get access to it should run _after_ this set in
/// [`CoreSet::PreUpdate`].
#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]

pub struct SpawnClientsSet;

/// The system set where various facets of the client are updated. Systems that
/// modify chunks should run _before_ this.
#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct UpdateClientsSet;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            (
                initial_join.after(RegistrySet),
                update_chunk_load_dist,
                update_layer_view
                    .after(WriteUpdatePacketsToInstancesSet)
                    .after(update_chunk_load_dist),
                read_data_in_old_view.after(update_layer_view),
                update_view.after(initial_join).after(read_data_in_old_view),
                respawn.after(update_view),
                remove_entities.after(update_view),
                update_spawn_position.after(update_view),
                update_old_view_dist.after(update_view),
                update_game_mode,
                update_tracked_data.after(WriteUpdatePacketsToInstancesSet),
                init_tracked_data.after(WriteUpdatePacketsToInstancesSet),
            )
                .in_set(UpdateClientsSet),
        )
        .configure_sets((
            SpawnClientsSet.in_base_set(CoreSet::PreUpdate),
            UpdateClientsSet
                .in_base_set(CoreSet::PostUpdate)
                .before(FlushPacketsSet),
            ClearEntityChangesSet.after(UpdateClientsSet),
            FlushPacketsSet.in_base_set(CoreSet::PostUpdate),
            ClearInstanceChangesSet.after(FlushPacketsSet),
        ))
        .add_system(flush_packets.in_set(FlushPacketsSet));

        event_loop::build(app);
        movement::build(app);
        command::build(app);
        keepalive::build(app);
        interact_entity::build(app);
        settings::build(app);
        action::build(app);
        teleport::build(app);
        weather::build(app);
        message::build(app);
        custom_payload::build(app);
        hand_swing::build(app);
        interact_block::build(app);
        interact_item::build(app);
        op_level::build(app);
        resource_pack::build(app);
        status::build(app);
        layer::build(app);
    }
}

/// The bundle of components needed for clients to function. All components are
/// required unless otherwise stated.
#[derive(Bundle)]
pub struct ClientBundle {
    pub client: Client,
    pub settings: settings::ClientSettings,
    pub scratch: ScratchBuf,
    pub entity_remove_buf: EntityRemoveBuf,
    pub username: Username,
    pub ip: Ip,
    pub properties: Properties,
    pub compass_pos: CompassPos,
    pub game_mode: GameMode,
    pub op_level: op_level::OpLevel,
    pub action_sequence: action::ActionSequence,
    pub view_distance: ViewDistance,
    pub old_view_distance: OldViewDistance,
    pub death_location: DeathLocation,
    pub keepalive_state: keepalive::KeepaliveState,
    pub ping: Ping,
    pub is_hardcore: IsHardcore,
    pub prev_game_mode: PrevGameMode,
    pub hashed_seed: HashedSeed,
    pub reduced_debug_info: ReducedDebugInfo,
    pub has_respawn_screen: HasRespawnScreen,
    pub is_debug: IsDebug,
    pub is_flat: IsFlat,
    pub teleport_state: teleport::TeleportState,
    pub player: PlayerEntityBundle,
}

impl ClientBundle {
    pub fn new(args: ClientBundleArgs) -> Self {
        Self {
            client: Client {
                conn: args.conn,
                enc: args.enc,
            },
            settings: settings::ClientSettings::default(),
            scratch: ScratchBuf::default(),
            entity_remove_buf: EntityRemoveBuf(vec![]),
            username: Username(args.username),
            ip: Ip(args.ip),
            properties: Properties(args.properties),
            compass_pos: CompassPos::default(),
            game_mode: GameMode::default(),
            op_level: op_level::OpLevel::default(),
            action_sequence: action::ActionSequence::default(),
            view_distance: ViewDistance::default(),
            old_view_distance: OldViewDistance(2),
            death_location: DeathLocation::default(),
            keepalive_state: keepalive::KeepaliveState::new(),
            ping: Ping::default(),
            teleport_state: teleport::TeleportState::new(),
            is_hardcore: IsHardcore::default(),
            is_flat: IsFlat::default(),
            has_respawn_screen: HasRespawnScreen::default(),
            prev_game_mode: PrevGameMode::default(),
            hashed_seed: HashedSeed::default(),
            reduced_debug_info: ReducedDebugInfo::default(),
            is_debug: IsDebug::default(),
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
    fn write_packet<P>(&mut self, packet: &P)
    where
        P: Packet + Encode,
    {
        self.enc.write_packet(packet)
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
    pub fn kill(&mut self, message: impl Into<Text>) {
        self.write_packet(&DeathMessageS2c {
            player_id: VarInt(0),
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

/// The position that regular compass items will point to.
#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct CompassPos(pub BlockPos);

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
        ChunkView {
            pos: self.pos.chunk_pos(),
            dist: self.view_dist.0,
        }
    }
}

#[derive(WorldQuery, Copy, Clone, Debug)]
pub struct OldView {
    pub old_pos: &'static OldPosition,
    pub old_view_dist: &'static OldViewDistance,
}

impl OldViewItem<'_> {
    pub fn get(&self) -> ChunkView {
        ChunkView {
            pos: self.old_pos.chunk_pos(),
            dist: self.old_view_dist.0,
        }
    }
}

#[derive(Component, Clone, PartialEq, Eq, Default, Debug)]
pub struct DeathLocation(pub Option<(Ident<String>, BlockPos)>);

/// Delay measured in milliseconds. Negative values indicate absence.
#[derive(Component, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct Ping(pub i32);

impl Default for Ping {
    fn default() -> Self {
        Self(-1)
    }
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

#[derive(Component, Copy, Clone, PartialEq, Eq, Debug)]
pub struct HasRespawnScreen(pub bool);

impl Default for HasRespawnScreen {
    fn default() -> Self {
        Self(true)
    }
}

#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct IsDebug(pub bool);

/// Changes the perceived horizon line (used for superflat worlds).
#[derive(Component, Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct IsFlat(pub bool);

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
    codec: Res<RegistryCodec>,
    tags: Res<TagsRegistry>,
    mut clients: Query<ClientJoinQuery, Added<Client>>,
    instances: Query<&Instance>,
    mut commands: Commands,
) {
    for mut q in &mut clients {
        let Ok(instance) = instances.get(q.loc.0) else {
            warn!("client {:?} joined nonexistent instance {:?}", q.entity, q.loc.0);
            commands.entity(q.entity).remove::<Client>();
            continue
        };

        let dimension_names: Vec<Ident<Cow<str>>> = codec
            .registry(BiomeRegistry::KEY)
            .iter()
            .map(|value| value.name.as_str_ident().into())
            .collect();

        let dimension_name: Ident<Cow<str>> = instance.dimension_type_name().into();

        let last_death_location = q.death_loc.0.as_ref().map(|(id, pos)| GlobalPos {
            dimension_name: id.as_str_ident().into(),
            position: *pos,
        });

        // The login packet is prepended so that it's sent before all the other packets.
        // Some packets don't work corectly when sent before the game join packet.
        _ = q.client.enc.prepend_packet(&GameJoinS2c {
            entity_id: 0, // We reserve ID 0 for clients.
            is_hardcore: q.is_hardcore.0,
            game_mode: *q.game_mode,
            previous_game_mode: q.prev_game_mode.0.map(|g| g as i8).unwrap_or(-1),
            dimension_names,
            registry_codec: Cow::Borrowed(codec.cached_codec()),
            dimension_type_name: dimension_name.clone(),
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
            portal_cooldown: VarInt(0), // TODO.
        });

        q.client.enc.append_bytes(tags.sync_tags_packet());

        /*
        // TODO: enable all the features?
        q.client.write_packet(&FeatureFlags {
            features: vec![Ident::new("vanilla").unwrap()],
        })?;
        */
    }
}

#[allow(clippy::type_complexity)]
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

        let dimension_name = instance.dimension_type_name();

        let last_death_location = death_loc.0.as_ref().map(|(id, pos)| GlobalPos {
            dimension_name: id.as_str_ident().into(),
            position: *pos,
        });

        client.write_packet(&PlayerRespawnS2c {
            dimension_type_name: dimension_name.into(),
            dimension_name: dimension_name.into(),
            hashed_seed: hashed_seed.0,
            game_mode: *game_mode,
            previous_game_mode: prev_game_mode.0.map(|g| g as i8).unwrap_or(-1),
            is_debug: is_debug.0,
            is_flat: is_flat.0,
            copy_metadata: true,
            last_death_location,
            portal_cooldown: VarInt(0), // TODO
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

#[derive(WorldQuery)]
struct EntityInitQuery {
    entity_id: &'static EntityId,
    uuid: &'static UniqueId,
    kind: &'static EntityKind,
    look: &'static Look,
    head_yaw: &'static HeadYaw,
    on_ground: &'static OnGround,
    object_data: &'static ObjectData,
    velocity: &'static Velocity,
    tracked_data: &'static TrackedData,
}

impl EntityInitQueryItem<'_> {
    /// Writes the appropriate packets to initialize an entity. This will spawn
    /// the entity and initialize tracked data.
    fn write_init_packets(&self, pos: DVec3, mut writer: impl WritePacket) {
        match *self.kind {
            EntityKind::MARKER => {}
            EntityKind::EXPERIENCE_ORB => {
                writer.write_packet(&ExperienceOrbSpawnS2c {
                    entity_id: self.entity_id.get().into(),
                    position: pos,
                    count: self.object_data.0 as i16,
                });
            }
            EntityKind::PLAYER => {
                writer.write_packet(&PlayerSpawnS2c {
                    entity_id: self.entity_id.get().into(),
                    player_uuid: self.uuid.0,
                    position: pos,
                    yaw: ByteAngle::from_degrees(self.look.yaw),
                    pitch: ByteAngle::from_degrees(self.look.pitch),
                });

                // Player spawn packet doesn't include head yaw for some reason.
                writer.write_packet(&EntitySetHeadYawS2c {
                    entity_id: self.entity_id.get().into(),
                    head_yaw: ByteAngle::from_degrees(self.head_yaw.0),
                });
            }
            _ => writer.write_packet(&EntitySpawnS2c {
                entity_id: self.entity_id.get().into(),
                object_uuid: self.uuid.0,
                kind: self.kind.get().into(),
                position: pos,
                pitch: ByteAngle::from_degrees(self.look.pitch),
                yaw: ByteAngle::from_degrees(self.look.yaw),
                head_yaw: ByteAngle::from_degrees(self.head_yaw.0),
                data: self.object_data.0.into(),
                velocity: self.velocity.to_packet_units(),
            }),
        }

        if let Some(init_data) = self.tracked_data.init_data() {
            writer.write_packet(&EntityTrackerUpdateS2c {
                entity_id: self.entity_id.get().into(),
                metadata: init_data.into(),
            });
        }
    }
}

#[allow(clippy::type_complexity)]
fn read_data_in_old_view(
    mut clients: Query<(
        &mut Client,
        &mut EntityRemoveBuf,
        &Location,
        &OldLocation,
        &Position,
        &OldPosition,
        &OldViewDistance,
        Option<&PacketByteRange>,
        Option<&ClientLayerSet>,
    )>,
    instances: Query<&Instance>,
    entities: Query<(EntityInitQuery, &OldPosition, Option<&Layer>)>,
    entity_ids: Query<(&EntityId, Option<&Layer>)>,
) {
    clients.par_iter_mut().for_each_mut(
        |(
            mut client,
            mut remove_buf,
            loc,
            old_loc,
            pos,
            old_pos,
            old_view_dist,
            byte_range,
            client_layer_set,
        )| {
            let Ok(instance) = instances.get(old_loc.get()) else {
                return;
            };

            // Send instance-wide packet data.
            client.write_packet_bytes(&instance.packet_buf);

            // TODO: cache the chunk position?
            let old_chunk_pos = old_pos.chunk_pos();
            let new_chunk_pos = pos.chunk_pos();

            let view = ChunkView::new(old_chunk_pos, old_view_dist.0);

            // Iterate over all visible chunks from the previous tick.
            view.for_each(|pos| {
                if let Some(cell) = instance.partition.get(&pos) {
                    if cell.chunk_removed && cell.chunk.is_none() {
                        // Chunk was previously loaded and is now deleted.
                        client.write_packet(&UnloadChunkS2c { pos });
                    }

                    if let Some(chunk) = &cell.chunk {
                        chunk.mark_viewed();
                    }

                    // Send entity spawn packets for entities entering the client's view.
                    for &(id, src_pos) in &cell.incoming {
                        if src_pos.map_or(true, |p| !view.contains(p)) {
                            // The incoming entity originated from outside the view distance, so it
                            // must be spawned.
                            if let Ok((entity, old_pos, layer)) = entities.get(id) {
                                // Notice we are spawning the entity at its old position rather than
                                // the current position. This is because the client could also
                                // receive update packets for this entity this tick, which may
                                // include a relative entity movement.
                                // We only spawn the entity if it is in a layer that the client is
                                if let (Some(client_layer_set), Some(layer)) =
                                    (client_layer_set, layer)
                                {
                                    if client_layer_set.contains(&layer.0) {
                                        entity.write_init_packets(old_pos.get(), &mut client.enc);
                                    }
                                } else {
                                    entity.write_init_packets(old_pos.get(), &mut client.enc);
                                }
                            }
                        }
                    }

                    // Send entity despawn packets for entities exiting the client's view.
                    for &(id, dest_pos) in &cell.outgoing {
                        if dest_pos.map_or(true, |p| !view.contains(p)) {
                            // The outgoing entity moved outside the view distance, so it must be
                            // despawned if it was in the client's layer mask.
                            if let Ok((entity_id, layer)) = entity_ids.get(id) {
                                if let (Some(client_layer_set), Some(layer)) =
                                    (client_layer_set, layer)
                                {
                                    if client_layer_set.contains(&layer.0) {
                                        remove_buf.push(entity_id.get());
                                    }
                                } else {
                                    remove_buf.push(entity_id.get());
                                }
                            }
                        }
                    }

                    // Using the layer mask we gonna send the right layers buffers of bytes to the
                    // client
                    if let Some(client_layer_set) = client_layer_set {
                        client_layer_set.layers.iter().for_each(|layer| {
                            if let Some(packet_buf) = cell.layers_packet_buf.get(layer) {
                                client.write_packet_bytes(&packet_buf[..]);
                            }
                        });
                    }

                    // Send all data in the chunk's packet buffer to this client. This will update
                    // entities in the cell, spawn or update the chunk in the cell, or send any
                    // other packet data that was added here by users.
                    match byte_range {
                        Some(byte_range) if pos == new_chunk_pos && loc == old_loc => {
                            // Skip range of bytes for the client's own entity.
                            client.write_packet_bytes(&cell.packet_buf[..byte_range.0.start]);
                            client.write_packet_bytes(&cell.packet_buf[byte_range.0.end..]);
                        }
                        _ => {
                            client.write_packet_bytes(&cell.packet_buf);
                        }
                    }
                }
            });
        },
    );
}

fn update_layer_view(
    mut clients: Query<(
        &mut Client,
        &mut EntityRemoveBuf,
        &OldPosition,
        &OldViewDistance,
        &ClientLayerSet,
    )>,
    entities: Query<(EntityInitQuery, &OldPosition, &Layer)>,
) {
    clients.par_iter_mut().for_each_mut(
        |(mut client, mut remove_buf, old_pos, old_view_dist, client_layer_set)| {
            // TODO: cache the chunk position?
            let old_chunk_pos = old_pos.chunk_pos();

            let view = ChunkView::new(old_chunk_pos, old_view_dist.0);

            // Send entity spawn packets for entities that are in an entered layer and
            // already in the client's view.
            for (entity, &old_pos, layer) in entities.iter() {
                if view.contains(old_pos.chunk_pos())
                    && client_layer_set.added().any(|l| l == &layer.0)
                {
                    entity.write_init_packets(old_pos.get(), &mut client.enc);
                }
            }

            // Send entity despawn packets for entities that are in an exited layer and
            // still in the client's view.
            for (entity_init_item, &old_pos, layer) in entities.iter() {
                if view.contains(old_pos.chunk_pos())
                    && client_layer_set.removed().any(|l| l == &layer.0)
                {
                    remove_buf.push(entity_init_item.entity_id.get());
                }
            }
        },
    );
}

/// Updates the clients' view, i.e. the set of chunks that are visible from the
/// client's chunk position.
///
/// This handles the situation when a client changes instances or chunk
/// position. It must run after [`read_data_in_old_view`].
#[allow(clippy::type_complexity)]
fn update_view(
    mut clients: Query<
        (
            Entity,
            &mut Client,
            &mut ScratchBuf,
            &mut EntityRemoveBuf,
            &Location,
            &OldLocation,
            &Position,
            &OldPosition,
            &ViewDistance,
            &OldViewDistance,
            Option<&ClientLayerSet>,
        ),
        Or<(Changed<Location>, Changed<Position>, Changed<ViewDistance>)>,
    >,
    instances: Query<&Instance>,
    entities: Query<(EntityInitQuery, &Position, Option<&Layer>)>,
    entity_ids: Query<(&EntityId, Option<&Layer>)>,
) {
    clients.par_iter_mut().for_each_mut(
        |(
            entity,
            mut client,
            mut scratch,
            mut remove_buf,
            loc,
            old_loc,
            pos,
            old_pos,
            view_dist,
            old_view_dist,
            client_layer_set,
        )| {
            // TODO: cache chunk pos?
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
                                client.write_packet(&UnloadChunkS2c { pos });
                            }

                            // Unload all the entities in the cell.
                            for &id in &cell.entities {
                                // Skip client's own entity.
                                if id != entity {
                                    if let Ok((entity_id, layer)) = entity_ids.get(id) {
                                        if let (Some(client_layer_set), Some(layer)) =
                                            (client_layer_set, layer)
                                        {
                                            if client_layer_set.contains(&layer.0) {
                                                info!("removing entity: {:?}", entity_id.get());
                                                remove_buf.push(entity_id.get());
                                            }
                                        } else {
                                            remove_buf.push(entity_id.get());
                                        }
                                    }
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
                                    &mut scratch.0,
                                );

                                chunk.mark_viewed();
                            }

                            // Load all the entities in this cell.
                            for &id in &cell.entities {
                                // Skip client's own entity.
                                if id != entity {
                                    if let Ok((entity, pos, layer)) = entities.get(id) {
                                        if let (Some(layer_mask), Some(layer)) =
                                            (client_layer_set, layer)
                                        {
                                            if layer_mask.contains(&layer.0) {
                                                entity
                                                    .write_init_packets(pos.get(), &mut client.enc);
                                            }
                                        } else {
                                            entity.write_init_packets(pos.get(), &mut client.enc);
                                        }
                                    }
                                }
                            }
                        }
                    });
                } else {
                    warn!("Client entered nonexistent instance ({loc:?}).");
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
                                client.write_packet(&UnloadChunkS2c { pos });
                            }

                            // Unload all the entities in the cell.
                            for &id in &cell.entities {
                                if let Ok((entity_id, layer)) = entity_ids.get(id) {
                                    if let (Some(layer_mask), Some(layer)) =
                                        (client_layer_set, layer)
                                    {
                                        if layer_mask.contains(&layer.0) {
                                            remove_buf.push(entity_id.get());
                                        }
                                    } else {
                                        remove_buf.push(entity_id.get());
                                    }
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
                                    &mut scratch.0,
                                );

                                chunk.mark_viewed();
                            }

                            // Load all the entities in this cell.
                            for &id in &cell.entities {
                                if let Ok((entity, pos, layer)) = entities.get(id) {
                                    if let (Some(layer_mask), Some(layer)) =
                                        (client_layer_set, layer)
                                    {
                                        if layer_mask.contains(&layer.0) {
                                            entity.write_init_packets(pos.get(), &mut client.enc);
                                        }
                                    } else {
                                        entity.write_init_packets(pos.get(), &mut client.enc);
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

/// Sets the client's compass position.
///
/// This also closes the "downloading terrain" screen when first joining, so
/// it should happen after the initial chunks are written.
fn update_spawn_position(mut clients: Query<(&mut Client, &CompassPos), Changed<CompassPos>>) {
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
