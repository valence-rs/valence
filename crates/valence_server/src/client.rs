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
use tracing::warn;
use uuid::Uuid;
use valence_entity::player::PlayerEntityBundle;
use valence_entity::query::EntityInitQuery;
use valence_entity::tracked_data::TrackedData;
use valence_entity::{
    ClearEntityChangesSet, EntityId, EntityStatus, OldPosition, Position, Velocity,
};
use valence_math::{DVec3, Vec3};
use valence_protocol::encode::{PacketEncoder, WritePacket};
use valence_protocol::packets::play::chunk_biome_data_s2c::ChunkBiome;
use valence_protocol::packets::play::game_state_change_s2c::GameEventKind;
use valence_protocol::packets::play::particle_s2c::Particle;
use valence_protocol::packets::play::{
    ChunkBiomeDataS2c, ChunkLoadDistanceS2c, ChunkRenderDistanceCenterS2c, DeathMessageS2c,
    DisconnectS2c, EntitiesDestroyS2c, EntityStatusS2c, EntityTrackerUpdateS2c,
    EntityVelocityUpdateS2c, GameStateChangeS2c, ParticleS2c, PlaySoundS2c, UnloadChunkS2c,
};
use valence_protocol::sound::{Sound, SoundCategory, SoundId};
use valence_protocol::text::{IntoText, Text};
use valence_protocol::var_int::VarInt;
use valence_protocol::{BlockPos, ChunkPos, Encode, GameMode, Packet, PropertyValue};
use valence_registry::RegistrySet;
use valence_server_common::{Despawned, UniqueId};

use crate::layer::{ChunkLayer, EntityLayer, UpdateLayersPostClientSet, UpdateLayersPreClientSet};
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

/// The system set where various facets of the client are updated. Systems that
/// modify layers should run _before_ this.
#[derive(SystemSet, Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct UpdateClientsSet;

impl Plugin for ClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                (
                    crate::spawn::initial_join.after(RegistrySet),
                    update_chunk_load_dist,
                    handle_layer_messages.after(update_chunk_load_dist),
                    update_view_and_layers
                        .after(crate::spawn::initial_join)
                        .after(handle_layer_messages),
                    cleanup_chunks_after_client_despawn.after(update_view_and_layers),
                    crate::spawn::update_respawn_position.after(update_view_and_layers),
                    crate::spawn::respawn.after(crate::spawn::update_respawn_position),
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
    pub settings: crate::client_settings::ClientSettings,
    pub entity_remove_buf: EntityRemoveBuf,
    pub username: Username,
    pub ip: Ip,
    pub properties: Properties,
    pub respawn_pos: crate::spawn::RespawnPosition,
    pub op_level: crate::op_level::OpLevel,
    pub action_sequence: crate::action::ActionSequence,
    pub view_distance: ViewDistance,
    pub old_view_distance: OldViewDistance,
    pub visible_chunk_layer: VisibleChunkLayer,
    pub old_visible_chunk_layer: OldVisibleChunkLayer,
    pub visible_entity_layers: VisibleEntityLayers,
    pub old_visible_entity_layers: OldVisibleEntityLayers,
    pub keepalive_state: crate::keepalive::KeepaliveState,
    pub ping: crate::keepalive::Ping,
    pub teleport_state: crate::teleport::TeleportState,
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
            settings: Default::default(),
            entity_remove_buf: Default::default(),
            username: Username(args.username),
            ip: Ip(args.ip),
            properties: Properties(args.properties),
            respawn_pos: Default::default(),
            op_level: Default::default(),
            action_sequence: Default::default(),
            view_distance: Default::default(),
            old_view_distance: OldViewDistance(2),
            visible_chunk_layer: Default::default(),
            old_visible_chunk_layer: OldVisibleChunkLayer(Entity::PLACEHOLDER),
            visible_entity_layers: Default::default(),
            old_visible_entity_layers: OldVisibleEntityLayers(BTreeSet::new()),
            keepalive_state: crate::keepalive::KeepaliveState::new(),
            ping: Default::default(),
            teleport_state: crate::teleport::TeleportState::new(),
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
    pub uuid: Uuid,
    pub ip: IpAddr,
    pub properties: Vec<PropertyValue>,
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

#[derive(Component, Clone, PartialEq, Eq, Default, Debug)]
pub struct Username(pub String);

impl fmt::Display for Username {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Component, Clone, PartialEq, Eq, Default, Debug)]
pub struct Properties(pub Vec<PropertyValue>);

impl Properties {
    /// Finds the property with the name "textures".
    pub fn textures(&self) -> Option<&PropertyValue> {
        self.0.iter().find(|prop| prop.name == "textures")
    }

    /// Finds the property with the name "textures".
    pub fn textures_mut(&mut self) -> Option<&mut PropertyValue> {
        self.0.iter_mut().find(|prop| prop.name == "textures")
    }
}

impl From<Vec<PropertyValue>> for Properties {
    fn from(value: Vec<PropertyValue>) -> Self {
        Self(value)
    }
}

impl Deref for Properties {
    type Target = [PropertyValue];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Component, Clone, PartialEq, Eq, Debug)]
pub struct Ip(pub IpAddr);

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
        ChunkView::new(self.pos.to_chunk_pos(), self.view_dist.0)
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

/// A [`Component`] containing a handle to the [`ChunkLayer`] a client can
/// see.
///
/// A client can only see one chunk layer at a time. Mutating this component
/// will cause the client to respawn in the new chunk layer.
#[derive(Component, Copy, Clone, PartialEq, Eq, Debug)]
pub struct VisibleChunkLayer(pub Entity);

impl Default for VisibleChunkLayer {
    fn default() -> Self {
        Self(Entity::PLACEHOLDER)
    }
}

/// The value of [`VisibleChunkLayer`] from the end of the previous tick.
#[derive(Component, PartialEq, Eq, Debug)]
pub struct OldVisibleChunkLayer(Entity);

impl OldVisibleChunkLayer {
    pub fn get(&self) -> Entity {
        self.0
    }
}

/// A [`Component`] containing the set of [`EntityLayer`]s a client can see.
/// All Minecraft entities from all layers in this set are potentially visible
/// to the client.
///
/// This set can be mutated at any time to change which entity layers are
/// visible to the client. [`Despawned`] entity layers are automatically
/// removed.
#[derive(Component, Default, Debug)]
pub struct VisibleEntityLayers(pub BTreeSet<Entity>);

/// The value of [`VisibleEntityLayers`] from the end of the previous tick.
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
            let block_pos = BlockPos::from_pos(old_view.old_pos.get());
            let old_view = old_view.get();

            fn in_radius(p0: BlockPos, p1: BlockPos, radius_squared: u32) -> bool {
                let dist_squared =
                    (p1.x - p0.x).pow(2) + (p1.y - p0.y).pow(2) + (p1.z - p0.z).pow(2);

                dist_squared as u32 <= radius_squared
            }

            // Chunk layer messages
            if let Ok(chunk_layer) = chunk_layers.get(old_visible_chunk_layer.get()) {
                let messages = chunk_layer.messages();
                let bytes = messages.bytes();

                // Global messages
                for (msg, range) in messages.iter_global() {
                    match msg {
                        crate::layer::chunk::GlobalMsg::Packet => {
                            client.write_packet_bytes(&bytes[range]);
                        }
                        crate::layer::chunk::GlobalMsg::PacketExcept { except } => {
                            if self_entity != except {
                                client.write_packet_bytes(&bytes[range]);
                            }
                        }
                    }
                }

                let mut chunk_biome_buf = vec![];

                // Local messages
                messages.query_local(old_view, |msg, range| match msg {
                    crate::layer::chunk::LocalMsg::PacketAt { .. } => {
                        client.write_packet_bytes(&bytes[range]);
                    }
                    crate::layer::chunk::LocalMsg::PacketAtExcept { except, .. } => {
                        if self_entity != except {
                            client.write_packet_bytes(&bytes[range]);
                        }
                    }
                    crate::layer::chunk::LocalMsg::RadiusAt {
                        center,
                        radius_squared,
                    } => {
                        if in_radius(block_pos, center, radius_squared) {
                            client.write_packet_bytes(&bytes[range]);
                        }
                    }
                    crate::layer::chunk::LocalMsg::RadiusAtExcept {
                        center,
                        radius_squared,
                        except,
                    } => {
                        if self_entity != except && in_radius(block_pos, center, radius_squared) {
                            client.write_packet_bytes(&bytes[range]);
                        }
                    }
                    crate::layer::chunk::LocalMsg::ChangeBiome { pos } => {
                        chunk_biome_buf.push(ChunkBiome {
                            pos,
                            data: &bytes[range],
                        });
                    }
                    crate::layer::chunk::LocalMsg::ChangeChunkState { pos } => {
                        match &bytes[range] {
                            [ChunkLayer::LOAD, .., ChunkLayer::UNLOAD] => {
                                // Chunk is being loaded and unloaded on the
                                // same tick, so there's no need to do anything.
                                debug_assert!(chunk_layer.chunk(pos).is_none());
                            }
                            [.., ChunkLayer::LOAD | ChunkLayer::OVERWRITE] => {
                                // Load chunk.
                                let chunk = chunk_layer.chunk(pos).expect("chunk must exist");
                                chunk.write_init_packets(&mut *client, pos, chunk_layer.info());
                                chunk.inc_viewer_count();
                            }
                            [.., ChunkLayer::UNLOAD] => {
                                // Unload chunk.
                                client.write_packet(&UnloadChunkS2c { pos });
                                debug_assert!(chunk_layer.chunk(pos).is_none());
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
                            crate::layer::entity::GlobalMsg::Packet => {
                                client.write_packet_bytes(&bytes[range]);
                            }
                            crate::layer::entity::GlobalMsg::PacketExcept { except } => {
                                if self_entity != except {
                                    client.write_packet_bytes(&bytes[range]);
                                }
                            }
                            crate::layer::entity::GlobalMsg::DespawnLayer => {
                                // Remove this entity layer. The changes to the visible entity layer
                                // set will be detected by the `update_view_and_layers` system and
                                // despawning of entities will happen there.
                                visible_entity_layers.0.remove(&layer_id);
                            }
                        }
                    }

                    // Local messages
                    messages.query_local(old_view, |msg, range| match msg {
                        crate::layer::entity::LocalMsg::DespawnEntity { pos: _, dest_layer } => {
                            if !old_visible_entity_layers.0.contains(&dest_layer) {
                                let mut bytes = &bytes[range];

                                while let Ok(id) = bytes.read_i32::<NativeEndian>() {
                                    if self_entity_id.get() != id {
                                        remove_buf.push(id);
                                    }
                                }
                            }
                        }
                        crate::layer::entity::LocalMsg::DespawnEntityTransition {
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
                        crate::layer::entity::LocalMsg::SpawnEntity { pos: _, src_layer } => {
                            if !old_visible_entity_layers.0.contains(&src_layer) {
                                let mut bytes = &bytes[range];

                                while let Ok(u64) = bytes.read_u64::<NativeEndian>() {
                                    let entity = Entity::from_bits(u64);

                                    if self_entity != entity {
                                        if let Ok((init, old_pos)) = entities.get(entity) {
                                            remove_buf.send_and_clear(&mut *client);

                                            // Spawn at the entity's old position since we may get a
                                            // relative movement packet for this entity in a later
                                            // iteration of the loop.
                                            init.write_init_packets(old_pos.get(), &mut *client);
                                        }
                                    }
                                }
                            }
                        }
                        crate::layer::entity::LocalMsg::SpawnEntityTransition {
                            pos: _,
                            src_pos,
                        } => {
                            if !old_view.contains(src_pos) {
                                let mut bytes = &bytes[range];

                                while let Ok(u64) = bytes.read_u64::<NativeEndian>() {
                                    let entity = Entity::from_bits(u64);

                                    if self_entity != entity {
                                        if let Ok((init, old_pos)) = entities.get(entity) {
                                            remove_buf.send_and_clear(&mut *client);

                                            // Spawn at the entity's old position since we may get a
                                            // relative movement packet for this entity in a later
                                            // iteration of the loop.
                                            init.write_init_packets(old_pos.get(), &mut *client);
                                        }
                                    }
                                }
                            }
                        }
                        crate::layer::entity::LocalMsg::PacketAt { pos: _ } => {
                            client.write_packet_bytes(&bytes[range]);
                        }
                        crate::layer::entity::LocalMsg::PacketAtExcept { pos: _, except } => {
                            if self_entity != except {
                                client.write_packet_bytes(&bytes[range]);
                            }
                        }
                        crate::layer::entity::LocalMsg::RadiusAt {
                            center,
                            radius_squared,
                        } => {
                            if in_radius(block_pos, center, radius_squared) {
                                client.write_packet_bytes(&bytes[range]);
                            }
                        }
                        crate::layer::entity::LocalMsg::RadiusAtExcept {
                            center,
                            radius_squared,
                            except,
                        } => {
                            if self_entity != except && in_radius(block_pos, center, radius_squared)
                            {
                                client.write_packet_bytes(&bytes[range]);
                            }
                        }
                    });

                    remove_buf.send_and_clear(&mut *client);
                }
            }
        },
    );
}

pub(crate) fn update_view_and_layers(
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
            let view = ChunkView::new(ChunkPos::from_pos(pos.0), view_dist.0);
            let old_view = ChunkView::new(ChunkPos::from_pos(old_pos.get()), old_view_dist.0);

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

                remove_buf.send_and_clear(&mut *client);

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

                    remove_buf.send_and_clear(&mut *client);

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

pub(crate) fn update_game_mode(mut clients: Query<(&mut Client, &GameMode), Changed<GameMode>>) {
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
