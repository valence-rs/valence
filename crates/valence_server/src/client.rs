use std::borrow::Cow;
use std::fmt;
use std::net::IpAddr;
use std::time::Instant;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_ecs::query::WorldQuery;
use bevy_ecs::system::Command;
use bytes::{Bytes, BytesMut};
use derive_more::{Deref, DerefMut, From, Into};
use tracing::warn;
use uuid::Uuid;
use valence_entity::player::PlayerEntityBundle;
use valence_entity::tracked_data::TrackedData;
use valence_entity::{EntityStatus, OldPosition, Position, Velocity};
use valence_math::{DVec3, Vec3};
use valence_protocol::encode::{PacketEncoder, WritePacket};
use valence_protocol::packets::play::game_state_change_s2c::GameEventKind;
use valence_protocol::packets::play::particle_s2c::Particle;
use valence_protocol::packets::play::{
    DeathMessageS2c, DisconnectS2c, EntitiesDestroyS2c, EntityStatusS2c, EntityTrackerUpdateS2c,
    EntityVelocityUpdateS2c, GameStateChangeS2c, ParticleS2c, PlaySoundS2c,
};
use valence_protocol::profile::Property;
use valence_protocol::sound::{Sound, SoundCategory, SoundId};
use valence_protocol::text::{IntoText, Text};
use valence_protocol::var_int::VarInt;
use valence_protocol::{Encode, GameMode, Packet};
use valence_server_common::{Despawned, UniqueId};

use crate::layer::{OldVisibleLayers, VisibleLayers};
use crate::ChunkView;

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

#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct SelfTrackedDataSet;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ClientDespawnSettings>()
            .configure_set(PostUpdate, SelfTrackedDataSet.before(FlushPacketsSet))
            .add_systems(
                PostUpdate,
                (
                    flush_packets.in_set(FlushPacketsSet),
                    (init_self_tracked_data, update_self_tracked_data)
                        .chain()
                        .in_set(SelfTrackedDataSet),
                    despawn_disconnected_clients,
                ),
            );
    }
}

/// The bundle of components needed for clients to function. All components are
/// required unless otherwise stated.
#[derive(Bundle)]
pub struct ClientBundle {
    pub marker: ClientMarker,
    pub client: Client,
    pub visible_layers: VisibleLayers,
    pub old_visible_layers: OldVisibleLayers,
    pub settings: crate::client_settings::ClientSettings,
    pub entity_remove_buf: EntityRemoveBuf,
    pub username: Username,
    pub ip: Ip,
    pub properties: Properties,
    pub respawn_pos: crate::movement::RespawnPosition,
    pub op_level: crate::op_level::OpLevel,
    pub action_sequence: crate::action::ActionSequence,
    pub view_distance: ViewDistance,
    pub old_view_distance: OldViewDistance,
    pub keepalive_state: crate::keepalive::KeepaliveState,
    pub ping: crate::keepalive::Ping,
    pub teleport_state: crate::movement::TeleportState,
    pub game_mode: GameMode,
    pub prev_game_mode: crate::spawn::PrevGameMode,
    pub death_location: crate::spawn::DeathLocation,
    pub is_hardcore: crate::spawn::IsHardcore,
    pub hashed_seed: crate::spawn::HashedSeed,
    pub reduced_debug_info: crate::spawn::ReducedDebugInfo,
    pub has_respawn_screen: crate::spawn::HasRespawnScreen,
    pub is_debug: crate::spawn::IsDebug,
    pub is_flat: crate::spawn::IsFlat,
    pub portal_cooldown: crate::spawn::PortalCooldown,
    pub flying_speed: crate::abilities::FlyingSpeed,
    pub fov_modifier: crate::abilities::FovModifier,
    pub player_abilities_flags: crate::abilities::PlayerAbilitiesFlags,
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
            visible_layers: Default::default(),
            old_visible_layers: Default::default(),
            settings: Default::default(),
            entity_remove_buf: Default::default(),
            username: Username(args.username),
            ip: Ip(args.ip),
            properties: Properties(args.properties),
            respawn_pos: Default::default(),
            op_level: Default::default(),
            action_sequence: Default::default(),
            view_distance: Default::default(),
            old_view_distance: Default::default(),
            keepalive_state: Default::default(),
            ping: Default::default(),
            teleport_state: Default::default(),
            game_mode: GameMode::default(),
            prev_game_mode: Default::default(),
            death_location: Default::default(),
            is_hardcore: Default::default(),
            is_flat: Default::default(),
            has_respawn_screen: Default::default(),
            hashed_seed: Default::default(),
            reduced_debug_info: Default::default(),
            is_debug: Default::default(),
            portal_cooldown: Default::default(),
            flying_speed: Default::default(),
            fov_modifier: Default::default(),
            player_abilities_flags: Default::default(),
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
    /// UUID of the client.
    pub uuid: Uuid,
    /// IP address of the client.
    pub ip: IpAddr,
    /// Properties of this client from the game profile.
    pub properties: Vec<Property>,
    /// The abstract socket connection.
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
    pub(crate) enc: PacketEncoder,
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
    // Note: This is private because flushing packets early would make us miss
    // prepended packets.
    fn flush_packets(&mut self) -> anyhow::Result<()> {
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
            id: SoundId::Direct {
                id: sound.to_ident().into(),
                range: None,
            },
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
#[derive(Component, Default, Debug)]
pub struct EntityRemoveBuf(Vec<VarInt>);

impl EntityRemoveBuf {
    pub fn push(&mut self, entity_id: i32) {
        debug_assert!(
            entity_id != 0,
            "removing entity with protocol ID 0 (which should be reserved for clients)"
        );

        self.0.push(VarInt(entity_id));
    }

    /// Sends the entity remove packet and clears the buffer. Does nothing if
    /// the buffer is empty.
    pub fn send_and_clear(&mut self, mut w: impl WritePacket) {
        if !self.0.is_empty() {
            w.write_packet(&EntitiesDestroyS2c {
                entity_ids: Cow::Borrowed(&self.0),
            });

            self.0.clear();
        }
    }
}

#[derive(Component, Clone, PartialEq, Eq, Default, Debug, Deref)]
pub struct Username(pub String);

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Player properties from the game profile.
#[derive(Component, Clone, PartialEq, Eq, Default, Debug, Deref, DerefMut, From, Into)]
pub struct Properties(pub Vec<Property>);

impl Properties {
    /// Finds the property with the name "textures".
    pub fn textures(&self) -> Option<&Property> {
        self.0.iter().find(|p| p.name == "textures")
    }

    /// Finds the property with the name "textures" mutably.
    pub fn textures_mut(&mut self) -> Option<&mut Property> {
        self.0.iter_mut().find(|p| p.name == "textures")
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PropertyValue {
    pub value: String,
    pub signature: Option<String>,
}

#[derive(Component, Clone, PartialEq, Eq, Debug, Deref)]
pub struct Ip(pub IpAddr);

#[derive(Component, Clone, PartialEq, Eq, Debug, Deref)]
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
#[derive(Component, Clone, PartialEq, Eq, Debug, Deref)]
pub struct OldViewDistance(u8);

impl Default for OldViewDistance {
    fn default() -> Self {
        Self(2)
    }
}

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
        ChunkView::new(self.pos.0.into(), self.view_dist.0)
    }
}

#[derive(WorldQuery, Copy, Clone, Debug)]
pub struct OldView {
    pub old_pos: &'static OldPosition,
    pub old_view_dist: &'static OldViewDistance,
}

impl OldViewItem<'_> {
    pub fn get(&self) -> ChunkView {
        ChunkView::new(self.old_pos.get().into(), self.old_view_dist.0)
    }
}

#[derive(Resource, Debug)]
pub struct ClientDespawnSettings {
    /// If disconnected clients should automatically have the [`Despawned`]
    /// component added to them. Without this enabled, clients entities must be
    /// removed from the world manually.
    pub despawn_disconnected_clients: bool,
}

impl Default for ClientDespawnSettings {
    fn default() -> Self {
        Self {
            despawn_disconnected_clients: true,
        }
    }
}

/// A system for adding [`Despawned`] components to disconnected clients. This
/// works by listening for removed [`Client`] components.
fn despawn_disconnected_clients(
    mut commands: Commands,
    mut disconnected_clients: RemovedComponents<Client>,
    cfg: Res<ClientDespawnSettings>,
) {
    if cfg.despawn_disconnected_clients {
        for entity in disconnected_clients.iter() {
            if let Some(mut entity) = commands.get_entity(entity) {
                entity.insert(Despawned);
            }
        }
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

fn init_self_tracked_data(mut clients: Query<(&mut Client, &TrackedData), Added<TrackedData>>) {
    for (mut client, tracked_data) in &mut clients {
        if let Some(init_data) = tracked_data.init_data() {
            client.write_packet(&EntityTrackerUpdateS2c {
                entity_id: VarInt(0),
                tracked_values: init_data.into(),
            });
        }
    }
}

fn update_self_tracked_data(mut clients: Query<(&mut Client, &TrackedData)>) {
    for (mut client, tracked_data) in &mut clients {
        if let Some(update_data) = tracked_data.update_data() {
            client.write_packet(&EntityTrackerUpdateS2c {
                entity_id: VarInt(0),
                tracked_values: update_data.into(),
            });
        }
    }
}
