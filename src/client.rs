use std::collections::HashSet;
use std::iter::FusedIterator;
use std::ops::Deref;

use flume::{Receiver, Sender, TrySendError};
use rayon::iter::ParallelIterator;
use uuid::Uuid;
use vek::Vec3;

use crate::biome::{Biome, BiomeGrassColorModifier, BiomePrecipitation};
use crate::block_pos::BlockPos;
use crate::byte_angle::ByteAngle;
use crate::dimension::{Dimension, DimensionEffects};
use crate::entity::{velocity_to_packet_units, EntityType};
use crate::packets::play::c2s::C2sPlayPacket;
pub use crate::packets::play::s2c::GameMode;
use crate::packets::play::s2c::{
    Biome as BiomeRegistryBiome, BiomeAdditionsSound, BiomeEffects, BiomeMoodSound, BiomeMusic,
    BiomeParticle, BiomeParticleOptions, BiomeProperty, BiomeRegistry, ChangeGameState,
    ChangeGameStateReason, ChatTypeRegistry, DestroyEntities, DimensionType, DimensionTypeRegistry,
    DimensionTypeRegistryEntry, Disconnect, EntityHeadLook, EntityPosition,
    EntityPositionAndRotation, EntityRotation, EntityTeleport, EntityVelocity, JoinGame,
    KeepAliveClientbound, PlayerPositionAndLook, PlayerPositionAndLookFlags, RegistryCodec,
    S2cPlayPacket, SpawnPosition, UnloadChunk, UpdateViewDistance, UpdateViewPosition,
};
use crate::protocol::{BoundedInt, Nbt};
use crate::server::C2sPacketChannels;
use crate::slotmap::{Key, SlotMap};
use crate::util::{chunks_in_view_distance, is_chunk_in_view_distance};
use crate::var_int::VarInt;
use crate::{
    ident, ChunkPos, Chunks, DimensionId, Entities, EntityId, Server, SpatialIndex, Text, Ticks,
    WorldMeta, LIBRARY_NAMESPACE,
};

pub struct Clients {
    sm: SlotMap<Client>,
}

pub struct ClientsMut<'a>(&'a mut Clients);

impl<'a> Deref for ClientsMut<'a> {
    type Target = Clients;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Clients {
    pub(crate) fn new() -> Self {
        Self { sm: SlotMap::new() }
    }

    pub fn count(&self) -> usize {
        self.sm.len()
    }

    pub fn get(&self, client: ClientId) -> Option<&Client> {
        self.sm.get(client.0)
    }

    pub fn iter(&self) -> impl FusedIterator<Item = (ClientId, &Client)> + Clone + '_ {
        self.sm.iter().map(|(k, v)| (ClientId(k), v))
    }

    pub fn par_iter(&self) -> impl ParallelIterator<Item = (ClientId, &Client)> + Clone + '_ {
        self.sm.par_iter().map(|(k, v)| (ClientId(k), v))
    }
}

impl<'a> ClientsMut<'a> {
    pub(crate) fn new(c: &'a mut Clients) -> Self {
        Self(c)
    }

    pub fn reborrow(&mut self) -> ClientsMut {
        ClientsMut(self.0)
    }

    pub(crate) fn create(&mut self, client: Client) -> (ClientId, ClientMut) {
        let (id, client) = self.0.sm.insert(client);
        (ClientId(id), ClientMut(client))
    }

    pub fn delete(&mut self, client: ClientId) -> bool {
        self.0.sm.remove(client.0).is_some()
    }

    pub fn retain(&mut self, mut f: impl FnMut(ClientId, ClientMut) -> bool) {
        self.0.sm.retain(|k, v| f(ClientId(k), ClientMut(v)))
    }

    pub fn get_mut(&mut self, client: ClientId) -> Option<ClientMut> {
        self.0.sm.get_mut(client.0).map(ClientMut)
    }

    pub fn iter_mut(&mut self) -> impl FusedIterator<Item = (ClientId, ClientMut)> + '_ {
        self.0
            .sm
            .iter_mut()
            .map(|(k, v)| (ClientId(k), ClientMut(v)))
    }

    pub fn par_iter_mut(&mut self) -> impl ParallelIterator<Item = (ClientId, ClientMut)> + '_ {
        self.0
            .sm
            .par_iter_mut()
            .map(|(k, v)| (ClientId(k), ClientMut(v)))
    }
}
pub struct ClientId(Key);

/// Represents a client connected to the server after logging in.
pub struct Client {
    /// Setting this to `None` disconnects the client.
    send: Option<Sender<S2cPlayPacket>>,
    recv: Receiver<C2sPlayPacket>,
    /// The tick this client was created.
    created_tick: Ticks,
    username: String,
    uuid: Uuid,
    on_ground: bool,
    new_position: Vec3<f64>,
    old_position: Vec3<f64>,
    /// Measured in degrees
    yaw: f32,
    /// Measured in degrees
    pitch: f32,
    /// If any of position, yaw, or pitch were modified by the
    /// user this tick.
    teleported_this_tick: bool,
    /// Counts up as teleports are made.
    teleport_id_counter: u32,
    /// The number of pending client teleports that have yet to receive a
    /// confirmation. Inbound client position packets are ignored while this
    /// is nonzero.
    pending_teleports: u32,
    spawn_position: BlockPos,
    spawn_position_yaw: f32,
    /// If spawn_position or spawn_position_yaw were modified this tick.
    modified_spawn_position: bool,
    death_location: Option<(DimensionId, BlockPos)>,
    events: Vec<Event>,
    /// The ID of the last keepalive sent.
    last_keepalive_id: i64,
    /// If the last sent keepalive got a response.
    got_keepalive: bool,
    new_max_view_distance: u8,
    old_max_view_distance: u8,
    /// Entities that were visible to this client at the end of the last tick.
    /// This is used to determine what entity create/destroy packets should be
    /// sent.
    loaded_entities: HashSet<EntityId>,
    loaded_chunks: HashSet<ChunkPos>,
    new_game_mode: GameMode,
    old_game_mode: GameMode,
    settings: Option<Settings>,
    // TODO: latency
    // TODO: time, weather
}

pub struct ClientMut<'a>(&'a mut Client);

impl<'a> Deref for ClientMut<'a> {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Client {
    pub(crate) fn new(
        packet_channels: C2sPacketChannels,
        username: String,
        uuid: Uuid,
        server: &Server,
    ) -> Self {
        let (send, recv) = packet_channels;

        Self {
            send: Some(send),
            recv,
            created_tick: server.current_tick(),
            username,
            uuid,
            on_ground: false,
            new_position: Vec3::default(),
            old_position: Vec3::default(),
            yaw: 0.0,
            pitch: 0.0,
            teleported_this_tick: false,
            teleport_id_counter: 0,
            pending_teleports: 0,
            spawn_position: BlockPos::default(),
            spawn_position_yaw: 0.0,
            modified_spawn_position: true,
            death_location: None,
            events: Vec::new(),
            last_keepalive_id: 0,
            got_keepalive: true,
            new_max_view_distance: 16,
            old_max_view_distance: 0,
            loaded_entities: HashSet::new(),
            loaded_chunks: HashSet::new(),
            new_game_mode: GameMode::Survival,
            old_game_mode: GameMode::Survival,
            settings: None,
        }
    }

    pub fn created_tick(&self) -> Ticks {
        self.created_tick
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    pub fn position(&self) -> Vec3<f64> {
        self.new_position
    }

    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// Gets the spawn position. The client will see regular compasses point at
    /// the returned position.
    pub fn spawn_position(&self) -> BlockPos {
        self.spawn_position
    }

    /// Gets the last death location. The client will see recovery compasses
    /// point at the returned position. If the client's current dimension
    /// differs from the returned dimension or the location is `None` then the
    /// compass will spin randomly.
    pub fn death_location(&self) -> Option<(DimensionId, BlockPos)> {
        self.death_location
    }

    pub fn game_mode(&self) -> GameMode {
        self.new_game_mode
    }

    pub fn on_ground(&self) -> bool {
        self.on_ground
    }

    pub fn is_disconnected(&self) -> bool {
        self.send.is_none()
    }

    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// The current view distance of this client measured in chunks.
    pub fn view_distance(&self) -> u8 {
        self.settings
            .as_ref()
            .map_or(2, |s| s.view_distance)
            .min(self.max_view_distance())
    }

    pub fn max_view_distance(&self) -> u8 {
        self.new_max_view_distance
    }

    pub fn settings(&self) -> Option<&Settings> {
        self.settings.as_ref()
    }
}

impl<'a> ClientMut<'a> {
    pub(crate) fn new(client: &'a mut Client) -> Self {
        Self(client)
    }

    pub fn reborrow(&mut self) -> ClientMut {
        ClientMut(self.0)
    }

    pub fn teleport(&mut self, pos: impl Into<Vec3<f64>>, yaw: f32, pitch: f32) {
        self.0.new_position = pos.into();
        self.0.yaw = yaw;
        self.0.pitch = pitch;

        if !self.teleported_this_tick {
            self.0.teleported_this_tick = true;

            self.0.pending_teleports = match self.0.pending_teleports.checked_add(1) {
                Some(n) => n,
                None => {
                    self.disconnect("Too many pending teleports");
                    return;
                }
            };

            self.0.teleport_id_counter = self.0.teleport_id_counter.wrapping_add(1);
        }
    }

    pub fn set_game_mode(&mut self, new_game_mode: GameMode) {
        self.0.new_game_mode = new_game_mode;
    }

    /// Sets the spawn position. The client will see regular compasses point at
    /// the provided position.
    pub fn set_spawn_position(&mut self, pos: impl Into<BlockPos>, yaw_degrees: f32) {
        let pos = pos.into();
        if pos != self.0.spawn_position || yaw_degrees != self.0.spawn_position_yaw {
            self.0.spawn_position = pos;
            self.0.spawn_position_yaw = yaw_degrees;
            self.0.modified_spawn_position = true;
        }
    }

    /// Sets the last death location. The client will see recovery compasses
    /// point at the provided position. If the client's current dimension
    /// differs from the provided dimension or the location is `None` then the
    /// compass will spin randomly.
    ///
    /// Changes to the last death location take effect when the client
    /// (re)spawns.
    pub fn set_death_location(&mut self, location: Option<(DimensionId, BlockPos)>) {
        self.0.death_location = location;
    }

    /// Attempts to enqueue a play packet to be sent to this client. The client
    /// is disconnected if the clientbound packet buffer is full.
    pub(crate) fn send_packet(&mut self, packet: impl Into<S2cPlayPacket>) {
        send_packet(&mut self.0.send, packet);
    }

    pub fn disconnect(&mut self, reason: impl Into<Text>) {
        if self.0.send.is_some() {
            let txt = reason.into();
            log::info!("disconnecting client '{}': \"{txt}\"", self.0.username);

            self.send_packet(Disconnect { reason: txt });

            self.0.send = None;
        }
    }

    pub fn disconnect_no_reason(&mut self) {
        if self.0.send.is_some() {
            log::info!("disconnecting client '{}' (no reason)", self.0.username);
            self.0.send = None;
        }
    }

    /// The new view distance is clamped to `2..=32`.
    pub fn set_max_view_distance(&mut self, dist: u8) {
        self.0.new_max_view_distance = dist.clamp(2, 32);
    }

    pub(crate) fn update(
        &mut self,
        server: &Server,
        entities: &Entities,
        spatial_index: &SpatialIndex,
        chunks: &Chunks,
        meta: &WorldMeta,
    ) {
        self.0.events.clear();

        if self.is_disconnected() {
            return;
        }

        for _ in 0..self.recv.len() {
            self.handle_serverbound_packet(self.recv.try_recv().unwrap());
        }

        // Mark the client as disconnected when appropriate.
        // We do this check after handling serverbound packets so that none are lost.
        if self.recv.is_disconnected() || self.send.as_ref().map_or(true, |s| s.is_disconnected()) {
            self.0.send = None;
            return;
        }

        let current_tick = server.current_tick();

        // Send the join game packet and other initial packets. We defer this until now
        // so that the user can set the client's location, game mode, etc.
        if self.created_tick == current_tick {
            self.send_packet(JoinGame {
                entity_id: 0,       // EntityId 0 is reserved for clients.
                is_hardcore: false, // TODO
                gamemode: self.new_game_mode,
                previous_gamemode: self.old_game_mode,
                dimension_names: server
                    .dimensions()
                    .map(|(id, _)| ident!("{LIBRARY_NAMESPACE}:dimension_{}", id.0))
                    .collect(),
                registry_codec: Nbt(make_dimension_codec(server)),
                dimension_type_name: ident!(
                    "{LIBRARY_NAMESPACE}:dimension_type_{}",
                    meta.dimension().0
                ),
                dimension_name: ident!("{LIBRARY_NAMESPACE}:dimension_{}", meta.dimension().0),
                hashed_seed: 0,
                max_players: VarInt(0),
                view_distance: BoundedInt(VarInt(self.new_max_view_distance as i32)),
                simulation_distance: VarInt(16),
                reduced_debug_info: false, // TODO
                enable_respawn_screen: false,
                is_debug: false,
                is_flat: meta.is_flat(),
                last_death_location: self
                    .death_location
                    .map(|(id, pos)| (ident!("{LIBRARY_NAMESPACE}:dimension_{}", id.0), pos)),
            });

            self.teleport(self.position(), self.yaw(), self.pitch());
        } else if self.0.old_game_mode != self.0.new_game_mode {
            self.0.old_game_mode = self.0.new_game_mode;
            self.send_packet(ChangeGameState {
                reason: ChangeGameStateReason::ChangeGameMode,
                value: self.0.new_game_mode as i32 as f32,
            });
        }

        // Update the players spawn position (compass position)
        if self.0.modified_spawn_position {
            self.0.modified_spawn_position = false;

            self.send_packet(SpawnPosition {
                location: self.spawn_position,
                angle: self.spawn_position_yaw,
            })
        }

        // Update view distance fog on the client if necessary.
        if self.0.old_max_view_distance != self.0.new_max_view_distance {
            self.0.old_max_view_distance = self.0.new_max_view_distance;
            if self.0.created_tick != current_tick {
                self.send_packet(UpdateViewDistance {
                    view_distance: BoundedInt(VarInt(self.0.new_max_view_distance as i32)),
                })
            }
        }

        // Check if it's time to send another keepalive.
        if current_tick % (server.tick_rate() * 8) == 0 {
            if self.0.got_keepalive {
                let id = rand::random();
                self.send_packet(KeepAliveClientbound { id });
                self.0.last_keepalive_id = id;
                self.0.got_keepalive = false;
            } else {
                self.disconnect("Timed out (no keepalive response)");
            }
        }

        let view_dist = self.view_distance();

        let center = ChunkPos::new(
            (self.new_position.x / 16.0) as i32,
            (self.new_position.z / 16.0) as i32,
        );

        // Send the update view position packet if the client changes the chunk section
        // they're in.
        {
            let old_section = self.0.old_position.map(|n| (n / 16.0) as i32);
            let new_section = self.0.new_position.map(|n| (n / 16.0) as i32);

            if old_section != new_section {
                self.send_packet(UpdateViewPosition {
                    chunk_x: VarInt(new_section.x),
                    chunk_z: VarInt(new_section.z),
                })
            }
        }

        let dimension = server.dimension(meta.dimension());

        // Update existing chunks and unload those outside the view distance. Chunks
        // that have been overwritten also need to be unloaded.
        self.0.loaded_chunks.retain(|&pos| {
            // The cache stops chunk data packets from needing to be sent when a player
            // moves to an adjacent chunk and back to the original.
            let cache = 2;

            if let Some(chunk) = chunks.get(pos) {
                if is_chunk_in_view_distance(center, pos, view_dist + cache)
                    && chunk.created_tick() != current_tick
                {
                    chunk.block_change_packets(pos, dimension.min_y, |pkt| {
                        send_packet(&mut self.0.send, pkt)
                    });
                    return true;
                }
            }

            send_packet(
                &mut self.0.send,
                UnloadChunk {
                    chunk_x: pos.x,
                    chunk_z: pos.z,
                },
            );
            false
        });

        // Load new chunks within the view distance
        for pos in chunks_in_view_distance(center, view_dist) {
            if let Some(chunk) = chunks.get(pos) {
                if self.0.loaded_chunks.insert(pos) {
                    self.send_packet(chunk.chunk_data_packet(pos));
                    chunk.block_change_packets(pos, dimension.min_y, |pkt| self.send_packet(pkt));
                }
            }
        }

        // This is done after the chunks are loaded so that the "downloading terrain"
        // screen is closed at the appropriate time.
        if self.0.teleported_this_tick {
            self.0.teleported_this_tick = false;

            self.send_packet(PlayerPositionAndLook {
                position: self.new_position,
                yaw: self.yaw,
                pitch: self.pitch,
                flags: PlayerPositionAndLookFlags::new(false, false, false, false, false),
                teleport_id: VarInt((self.teleport_id_counter - 1) as i32),
                dismount_vehicle: false,
            });
        }

        let mut entities_to_unload = Vec::new();

        // Update all entities that are visible and unload entities that are no
        // longer visible.
        self.0.loaded_entities.retain(|&id| {
            if let Some(entity) = entities.get(id) {
                debug_assert!(entity.typ() != EntityType::Marker);
                if self.0.new_position.distance(entity.position()) <= view_dist as f64 * 16.0
                    && !entity.flags().type_modified()
                {
                    if let Some(meta) = entity.updated_metadata_packet(id) {
                        send_packet(&mut self.0.send, meta);
                    }

                    let position_delta = entity.position() - entity.old_position();
                    let needs_teleport = position_delta.map(f64::abs).reduce_partial_max() >= 8.0;
                    let flags = entity.flags();

                    if entity.position() != entity.old_position()
                        && !needs_teleport
                        && flags.yaw_or_pitch_modified()
                    {
                        send_packet(
                            &mut self.0.send,
                            EntityPositionAndRotation {
                                entity_id: VarInt(id.to_network_id()),
                                delta: (position_delta * 4096.0).as_(),
                                yaw: ByteAngle::from_degrees(entity.yaw()),
                                pitch: ByteAngle::from_degrees(entity.pitch()),
                                on_ground: entity.on_ground(),
                            },
                        );
                    } else {
                        if entity.position() != entity.old_position() && !needs_teleport {
                            send_packet(
                                &mut self.0.send,
                                EntityPosition {
                                    entity_id: VarInt(id.to_network_id()),
                                    delta: (position_delta * 4096.0).as_(),
                                    on_ground: entity.on_ground(),
                                },
                            );
                        }

                        if flags.yaw_or_pitch_modified() {
                            send_packet(
                                &mut self.0.send,
                                EntityRotation {
                                    entity_id: VarInt(id.to_network_id()),
                                    yaw: ByteAngle::from_degrees(entity.yaw()),
                                    pitch: ByteAngle::from_degrees(entity.pitch()),
                                    on_ground: entity.on_ground(),
                                },
                            );
                        }
                    }

                    if needs_teleport {
                        send_packet(
                            &mut self.0.send,
                            EntityTeleport {
                                entity_id: VarInt(id.to_network_id()),
                                position: entity.position(),
                                yaw: ByteAngle::from_degrees(entity.yaw()),
                                pitch: ByteAngle::from_degrees(entity.pitch()),
                                on_ground: entity.on_ground(),
                            },
                        );
                    }

                    if flags.velocity_modified() {
                        send_packet(
                            &mut self.0.send,
                            EntityVelocity {
                                entity_id: VarInt(id.to_network_id()),
                                velocity: velocity_to_packet_units(entity.velocity()),
                            },
                        );
                    }

                    if flags.head_yaw_modified() {
                        send_packet(
                            &mut self.0.send,
                            EntityHeadLook {
                                entity_id: VarInt(id.to_network_id()),
                                head_yaw: ByteAngle::from_degrees(entity.head_yaw()),
                            },
                        )
                    }

                    return true;
                }
            }

            entities_to_unload.push(VarInt(id.to_network_id()));
            false
        });

        if !entities_to_unload.is_empty() {
            self.send_packet(DestroyEntities {
                entities: entities_to_unload,
            });
        }

        // Spawn new entities within the view distance.
        let pos = self.position();
        spatial_index.query::<_, _, ()>(
            |bb| bb.projected_point(pos).distance(pos) <= view_dist as f64 * 16.0,
            |id, _| {
                if self.0.loaded_entities.insert(id) {
                    let entity = entities.get(id).unwrap();
                    if entity.typ() != EntityType::Marker {
                        self.send_packet(
                            entity
                                .spawn_packet(id)
                                .expect("should not be a marker entity"),
                        );

                        if let Some(meta) = entity.initial_metadata_packet(id) {
                            self.send_packet(meta);
                        }
                    }
                }
                None
            },
        );

        self.0.old_position = self.0.new_position;
    }

    fn handle_serverbound_packet(&mut self, pkt: C2sPlayPacket) {
        let client = &mut self.0;

        fn handle_movement_packet(
            client: &mut Client,
            new_position: Vec3<f64>,
            new_yaw: f32,
            new_pitch: f32,
            new_on_ground: bool,
        ) {
            if client.pending_teleports == 0 {
                let event = Event::Movement {
                    position: client.new_position,
                    yaw: client.yaw,
                    pitch: client.pitch,
                    on_ground: client.on_ground,
                };

                client.new_position = new_position;
                client.yaw = new_yaw;
                client.pitch = new_pitch;
                client.on_ground = new_on_ground;

                client.events.push(event);
            }
        }

        match pkt {
            C2sPlayPacket::TeleportConfirm(p) => {
                if client.pending_teleports == 0 {
                    self.disconnect("Unexpected teleport confirmation");
                    return;
                }

                let got = p.teleport_id.0 as u32;
                let expected = client
                    .teleport_id_counter
                    .wrapping_sub(client.pending_teleports);

                if got == expected {
                    client.pending_teleports -= 1;
                } else {
                    self.disconnect(format!(
                        "Unexpected teleport ID (expected {expected}, got {got})"
                    ));
                }
            }
            C2sPlayPacket::QueryBlockNbt(_) => {}
            C2sPlayPacket::SetDifficulty(_) => {}
            C2sPlayPacket::ChatMessageServerbound(_) => {}
            C2sPlayPacket::ChatPreview(_) => {}
            C2sPlayPacket::ClientStatus(_) => {}
            C2sPlayPacket::ClientSettings(p) => {
                let old = client.settings.replace(Settings {
                    locale: p.locale.0,
                    view_distance: p.view_distance.0,
                    chat_mode: p.chat_mode,
                    chat_colors: p.chat_colors,
                    main_hand: p.main_hand,
                    displayed_skin_parts: p.displayed_skin_parts,
                    allow_server_listings: p.allow_server_listings,
                });

                client.events.push(Event::SettingsChanged(old));
            }
            C2sPlayPacket::TabCompleteServerbound(_) => {}
            C2sPlayPacket::ClickWindowButton(_) => {}
            C2sPlayPacket::CloseWindow(_) => {}
            C2sPlayPacket::PluginMessageServerbound(_) => {}
            C2sPlayPacket::EditBook(_) => {}
            C2sPlayPacket::QueryEntityNbt(_) => {}
            C2sPlayPacket::InteractEntity(_) => {}
            C2sPlayPacket::GenerateStructure(_) => {}
            C2sPlayPacket::KeepAliveServerbound(p) => {
                let last_keepalive_id = client.last_keepalive_id;
                if client.got_keepalive {
                    self.disconnect("Unexpected keepalive");
                } else if p.id != last_keepalive_id {
                    self.disconnect(format!(
                        "Keepalive ids don't match (expected {}, got {})",
                        last_keepalive_id, p.id
                    ));
                } else {
                    client.got_keepalive = true;
                }
            }
            C2sPlayPacket::LockDifficulty(_) => {}
            C2sPlayPacket::PlayerPosition(p) => {
                handle_movement_packet(client, p.position, client.yaw, client.pitch, p.on_ground)
            }
            C2sPlayPacket::PlayerPositionAndRotation(p) => {
                handle_movement_packet(client, p.position, p.yaw, p.pitch, p.on_ground)
            }
            C2sPlayPacket::PlayerRotation(p) => {
                handle_movement_packet(client, client.new_position, p.yaw, p.pitch, p.on_ground)
            }

            C2sPlayPacket::PlayerMovement(p) => handle_movement_packet(
                client,
                client.new_position,
                client.yaw,
                client.pitch,
                p.on_ground,
            ),
            C2sPlayPacket::VehicleMoveServerbound(_) => {}
            C2sPlayPacket::SteerBoat(_) => {}
            C2sPlayPacket::PickItem(_) => {}
            C2sPlayPacket::CraftRecipeRequest(_) => {}
            C2sPlayPacket::PlayerAbilitiesServerbound(_) => {}
            C2sPlayPacket::PlayerDigging(_) => {}
            C2sPlayPacket::EntityAction(_) => {}
            C2sPlayPacket::SteerVehicle(_) => {}
            C2sPlayPacket::Pong(_) => {}
            C2sPlayPacket::SetRecipeBookState(_) => {}
            C2sPlayPacket::SetDisplayedRecipe(_) => {}
            C2sPlayPacket::NameItem(_) => {}
            C2sPlayPacket::ResourcePackStatus(_) => {}
            C2sPlayPacket::AdvancementTab(_) => {}
            C2sPlayPacket::SelectTrade(_) => {}
            C2sPlayPacket::SetBeaconEffect(_) => {}
            C2sPlayPacket::HeldItemChangeServerbound(_) => {}
            C2sPlayPacket::UpdateCommandBlock(_) => {}
            C2sPlayPacket::UpdateCommandBlockMinecart(_) => {}
            C2sPlayPacket::CreativeInventoryAction(_) => {}
            C2sPlayPacket::UpdateJigsawBlock(_) => {}
            C2sPlayPacket::UpdateStructureBlock(_) => {}
            C2sPlayPacket::UpdateSign(_) => {}
            C2sPlayPacket::PlayerArmSwing(_) => {}
            C2sPlayPacket::Spectate(_) => {}
            C2sPlayPacket::PlayerBlockPlacement(_) => {}
            C2sPlayPacket::UseItem(_) => {}
        }
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        log::trace!("Dropping client '{}'", self.username);
    }
}

#[derive(Debug)]
pub enum Event {
    /// Settings were changed. The value in this variant is the previous client
    /// settings.
    SettingsChanged(Option<Settings>),

    /// The client has moved. The values in this variant are the previous
    /// position and look.
    Movement {
        position: Vec3<f64>,
        yaw: f32,
        pitch: f32,
        on_ground: bool,
    },
}

#[derive(Clone, PartialEq, Debug)]
pub struct Settings {
    /// e.g. en_US
    pub locale: String,
    /// The client side render distance, in chunks.
    ///
    /// The value is always in `2..=32`.
    pub view_distance: u8,
    pub chat_mode: ChatMode,
    /// `true` if the client has chat colors enabled, `false` otherwise.
    pub chat_colors: bool,
    pub main_hand: MainHand,
    pub displayed_skin_parts: DisplayedSkinParts,
    pub allow_server_listings: bool,
}

pub use crate::packets::play::c2s::{ChatMode, DisplayedSkinParts, MainHand};

fn send_packet(send_opt: &mut Option<Sender<S2cPlayPacket>>, pkt: impl Into<S2cPlayPacket>) {
    if let Some(send) = send_opt {
        match send.try_send(pkt.into()) {
            Err(TrySendError::Full(_)) => {
                log::warn!("max outbound packet capacity reached for client");
                *send_opt = None;
            }
            Err(TrySendError::Disconnected(_)) => {
                *send_opt = None;
            }
            Ok(_) => {}
        }
    }
}

fn make_dimension_codec(server: &Server) -> RegistryCodec {
    let mut dims = Vec::new();
    for (id, dim) in server.dimensions() {
        let id = id.0 as i32;
        dims.push(DimensionTypeRegistryEntry {
            name: ident!("{LIBRARY_NAMESPACE}:dimension_type_{id}"),
            id,
            element: to_dimension_registry_item(dim),
        })
    }

    let mut biomes = Vec::new();
    for (id, biome) in server.biomes() {
        biomes.push(to_biome_registry_item(biome, id.0 as i32));
    }

    // The client needs a biome named "minecraft:plains" in the registry to
    // connect. This is probably a bug.
    //
    // If the issue is resolved, just delete this block.
    if !biomes.iter().any(|b| b.name == ident!("plains")) {
        let biome = Biome::default();
        assert_eq!(biome.name, ident!("plains"));
        biomes.push(to_biome_registry_item(&biome, 0));
    }

    RegistryCodec {
        dimension_type_registry: DimensionTypeRegistry {
            typ: ident!("dimension_type"),
            value: dims,
        },
        biome_registry: BiomeRegistry {
            typ: ident!("worldgen/biome"),
            value: biomes,
        },
        chat_type_registry: ChatTypeRegistry {
            typ: ident!("chat_type"),
            value: Vec::new(),
        },
    }
}

fn to_dimension_registry_item(dim: &Dimension) -> DimensionType {
    DimensionType {
        piglin_safe: true,
        has_raids: true,
        monster_spawn_light_level: 0,
        monster_spawn_block_light_limit: 0,
        natural: dim.natural,
        ambient_light: dim.ambient_light,
        fixed_time: dim.fixed_time.map(|t| t as i64),
        infiniburn: "#minecraft:infiniburn_overworld".into(),
        respawn_anchor_works: true,
        has_skylight: true,
        bed_works: true,
        effects: match dim.effects {
            DimensionEffects::Overworld => ident!("overworld"),
            DimensionEffects::TheNether => ident!("the_nether"),
            DimensionEffects::TheEnd => ident!("the_end"),
        },
        min_y: dim.min_y,
        height: dim.height,
        logical_height: dim.height,
        coordinate_scale: 1.0,
        ultrawarm: false,
        has_ceiling: false,
    }
}

fn to_biome_registry_item(biome: &Biome, id: i32) -> BiomeRegistryBiome {
    BiomeRegistryBiome {
        name: biome.name.clone(),
        id,
        element: BiomeProperty {
            precipitation: match biome.precipitation {
                BiomePrecipitation::Rain => "rain",
                BiomePrecipitation::Snow => "snow",
                BiomePrecipitation::None => "none",
            }
            .into(),
            depth: 0.125,
            temperature: 0.8,
            scale: 0.05,
            downfall: 0.4,
            category: "none".into(),
            temperature_modifier: None,
            effects: BiomeEffects {
                sky_color: biome.sky_color as i32,
                water_fog_color: biome.water_fog_color as i32,
                fog_color: biome.fog_color as i32,
                water_color: biome.water_color as i32,
                foliage_color: biome.foliage_color.map(|x| x as i32),
                grass_color: None,
                grass_color_modifier: match biome.grass_color_modifier {
                    BiomeGrassColorModifier::Swamp => Some("swamp".into()),
                    BiomeGrassColorModifier::DarkForest => Some("dark_forest".into()),
                    BiomeGrassColorModifier::None => None,
                },
                music: biome.music.as_ref().map(|bm| BiomeMusic {
                    replace_current_music: bm.replace_current_music,
                    sound: bm.sound.clone(),
                    max_delay: bm.max_delay,
                    min_delay: bm.min_delay,
                }),
                ambient_sound: biome.ambient_sound.clone(),
                additions_sound: biome.additions_sound.as_ref().map(|a| BiomeAdditionsSound {
                    sound: a.sound.clone(),
                    tick_chance: a.tick_chance,
                }),
                mood_sound: biome.mood_sound.as_ref().map(|m| BiomeMoodSound {
                    sound: m.sound.clone(),
                    tick_delay: m.tick_delay,
                    offset: m.offset,
                    block_search_extent: m.block_search_extent,
                }),
            },
            particle: biome.particle.as_ref().map(|p| BiomeParticle {
                probability: p.probability,
                options: BiomeParticleOptions { typ: p.typ.clone() },
            }),
        },
    }
}
