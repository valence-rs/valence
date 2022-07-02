/// Contains the [`Event`] enum and related data types.
mod event;
use std::collections::HashSet;
use std::iter::FusedIterator;
use std::time::Duration;

pub use event::*;
use flume::{Receiver, Sender, TrySendError};
use rayon::iter::ParallelIterator;
use uuid::Uuid;
use vek::Vec3;

use crate::biome::{Biome, BiomeGrassColorModifier, BiomePrecipitation};
use crate::dimension::{Dimension, DimensionEffects};
use crate::entity::types::Player;
use crate::entity::{velocity_to_packet_units, EntityType};
use crate::player_textures::SignedPlayerTextures;
use crate::protocol::packets::play::c2s::{C2sPlayPacket, DiggingStatus, InteractType};
use crate::protocol::packets::play::s2c::{
    Biome as BiomeRegistryBiome, BiomeAdditionsSound, BiomeEffects, BiomeMoodSound, BiomeMusic,
    BiomeParticle, BiomeParticleOptions, BiomeProperty, BiomeRegistry, BlockChangeAck, ChatType,
    ChatTypeChat, ChatTypeNarration, ChatTypeRegistry, ChatTypeRegistryEntry, DimensionType,
    DimensionTypeRegistry, DimensionTypeRegistryEntry, Disconnect, ForgetLevelChunk, GameEvent,
    GameEventReason, KeepAlive, Login, MoveEntityPosition, MoveEntityPositionAndRotation,
    MoveEntityRotation, PlayerPosition, PlayerPositionFlags, RegistryCodec, RemoveEntities,
    RotateHead, S2cPlayPacket, SetChunkCacheCenter, SetChunkCacheRadius, SetEntityMetadata,
    SetEntityMotion, SpawnPosition, SystemChat, TeleportEntity,
};
use crate::protocol::{BoundedInt, ByteAngle, Nbt, RawBytes, VarInt};
use crate::server::C2sPacketChannels;
use crate::slotmap::{Key, SlotMap};
use crate::util::{chunks_in_view_distance, is_chunk_in_view_distance};
use crate::{
    ident, BlockPos, ChunkPos, Chunks, DimensionId, Entities, EntityId, NewClientData, Server,
    SpatialIndex, Text, Ticks, WorldMeta, LIBRARY_NAMESPACE,
};

pub struct Clients {
    sm: SlotMap<Client>,
}

impl Clients {
    pub(crate) fn new() -> Self {
        Self { sm: SlotMap::new() }
    }

    pub(crate) fn create(&mut self, client: Client) -> (ClientId, &mut Client) {
        let (id, client) = self.sm.insert(client);
        (ClientId(id), client)
    }

    pub fn delete(&mut self, client: ClientId) -> bool {
        self.sm.remove(client.0).is_some()
    }

    pub fn retain(&mut self, mut f: impl FnMut(ClientId, &mut Client) -> bool) {
        self.sm.retain(|k, v| f(ClientId(k), v))
    }

    pub fn count(&self) -> usize {
        self.sm.len()
    }

    pub fn get(&self, client: ClientId) -> Option<&Client> {
        self.sm.get(client.0)
    }

    pub fn get_mut(&mut self, client: ClientId) -> Option<&mut Client> {
        self.sm.get_mut(client.0)
    }

    pub fn iter(&self) -> impl FusedIterator<Item = (ClientId, &Client)> + Clone + '_ {
        self.sm.iter().map(|(k, v)| (ClientId(k), v))
    }

    pub fn iter_mut(&mut self) -> impl FusedIterator<Item = (ClientId, &mut Client)> + '_ {
        self.sm.iter_mut().map(|(k, v)| (ClientId(k), v))
    }

    pub fn par_iter(&self) -> impl ParallelIterator<Item = (ClientId, &Client)> + Clone + '_ {
        self.sm.par_iter().map(|(k, v)| (ClientId(k), v))
    }

    pub fn par_iter_mut(&mut self) -> impl ParallelIterator<Item = (ClientId, &mut Client)> + '_ {
        self.sm.par_iter_mut().map(|(k, v)| (ClientId(k), v))
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
    uuid: Uuid,
    username: String,
    textures: Option<SignedPlayerTextures>,
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
    dug_blocks: Vec<i32>,
    msgs_to_send: Vec<Text>,
    /// The metadata for the client's own player entity.
    player_meta: Player,
}

impl Client {
    pub(crate) fn new(
        packet_channels: C2sPacketChannels,
        server: &Server,
        ncd: NewClientData,
    ) -> Self {
        let (send, recv) = packet_channels;

        Self {
            send: Some(send),
            recv,
            created_tick: server.current_tick(),
            uuid: ncd.uuid,
            username: ncd.username,
            textures: ncd.textures,
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
            dug_blocks: Vec::new(),
            msgs_to_send: Vec::new(),
            player_meta: Player::new(),
        }
    }

    pub fn created_tick(&self) -> Ticks {
        self.created_tick
    }

    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn textures(&self) -> Option<&SignedPlayerTextures> {
        self.textures.as_ref()
    }

    /// Sends a system message to the player.
    pub fn send_message(&mut self, msg: impl Into<Text>) {
        self.msgs_to_send.push(msg.into());
    }

    pub fn position(&self) -> Vec3<f64> {
        self.new_position
    }

    pub fn teleport(&mut self, pos: impl Into<Vec3<f64>>, yaw: f32, pitch: f32) {
        self.new_position = pos.into();

        self.yaw = yaw;
        self.pitch = pitch;

        if !self.teleported_this_tick {
            self.teleported_this_tick = true;

            self.pending_teleports = match self.pending_teleports.checked_add(1) {
                Some(n) => n,
                None => {
                    self.disconnect("Too many pending teleports");
                    return;
                }
            };

            self.teleport_id_counter = self.teleport_id_counter.wrapping_add(1);
        }
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

    /// Sets the spawn position. The client will see regular compasses point at
    /// the provided position.
    pub fn set_spawn_position(&mut self, pos: impl Into<BlockPos>, yaw_degrees: f32) {
        let pos = pos.into();
        if pos != self.spawn_position || yaw_degrees != self.spawn_position_yaw {
            self.spawn_position = pos;
            self.spawn_position_yaw = yaw_degrees;
            self.modified_spawn_position = true;
        }
    }

    /// Gets the last death location. The client will see recovery compasses
    /// point at the returned position. If the client's current dimension
    /// differs from the returned dimension or the location is `None` then the
    /// compass will spin randomly.
    pub fn death_location(&self) -> Option<(DimensionId, BlockPos)> {
        self.death_location
    }

    /// Sets the last death location. The client will see recovery compasses
    /// point at the provided position. If the client's current dimension
    /// differs from the provided dimension or the location is `None` then the
    /// compass will spin randomly.
    ///
    /// Changes to the last death location take effect when the client
    /// (re)spawns.
    pub fn set_death_location(&mut self, location: Option<(DimensionId, BlockPos)>) {
        self.death_location = location;
    }

    pub fn game_mode(&self) -> GameMode {
        self.new_game_mode
    }

    pub fn set_game_mode(&mut self, new_game_mode: GameMode) {
        self.new_game_mode = new_game_mode;
    }

    pub fn on_ground(&self) -> bool {
        self.on_ground
    }

    pub fn is_disconnected(&self) -> bool {
        self.send.is_none()
    }

    pub fn events(&self) -> &Vec<Event> {
        &self.events
    }

    pub fn events_mut(&mut self) -> &mut Vec<Event> {
        &mut self.events
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

    /// The new view distance is clamped to `2..=32`.
    pub fn set_max_view_distance(&mut self, dist: u8) {
        self.new_max_view_distance = dist.clamp(2, 32);
    }

    pub fn settings(&self) -> Option<&Settings> {
        self.settings.as_ref()
    }

    pub fn disconnect(&mut self, reason: impl Into<Text>) {
        if self.send.is_some() {
            let txt = reason.into();
            log::info!("disconnecting client '{}': \"{txt}\"", self.username);

            self.send_packet(Disconnect { reason: txt });

            self.send = None;
        }
    }

    pub fn disconnect_no_reason(&mut self) {
        if self.send.is_some() {
            log::info!("disconnecting client '{}' (no reason)", self.username);
            self.send = None;
        }
    }

    pub fn meta(&self) -> &Player {
        &self.player_meta
    }

    pub fn meta_mut(&mut self) -> &mut Player {
        &mut self.player_meta
    }

    /// Attempts to enqueue a play packet to be sent to this client. The client
    /// is disconnected if the clientbound packet buffer is full.
    pub(crate) fn send_packet(&mut self, packet: impl Into<S2cPlayPacket>) {
        send_packet(&mut self.send, packet);
    }

    pub(crate) fn handle_serverbound_packets(&mut self, entities: &Entities) {
        self.events.clear();
        for _ in 0..self.recv.len() {
            self.handle_serverbound_packet(entities, self.recv.try_recv().unwrap());
        }
    }

    fn handle_serverbound_packet(&mut self, entities: &Entities, pkt: C2sPlayPacket) {
        fn handle_movement_packet(
            client: &mut Client,
            _vehicle: bool,
            new_position: Vec3<f64>,
            new_yaw: f32,
            new_pitch: f32,
            new_on_ground: bool,
        ) {
            if client.pending_teleports == 0 {
                // TODO: validate movement using swept AABB collision with the blocks.
                // TODO: validate that the client is actually inside/outside the vehicle?
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
            C2sPlayPacket::AcceptTeleportation(p) => {
                if self.pending_teleports == 0 {
                    self.disconnect("Unexpected teleport confirmation");
                    return;
                }

                let got = p.teleport_id.0 as u32;
                let expected = self
                    .teleport_id_counter
                    .wrapping_sub(self.pending_teleports);

                if got == expected {
                    self.pending_teleports -= 1;
                } else {
                    self.disconnect(format!(
                        "Unexpected teleport ID (expected {expected}, got {got})"
                    ));
                }
            }
            C2sPlayPacket::BlockEntityTagQuery(_) => {}
            C2sPlayPacket::ChangeDifficulty(_) => {}
            C2sPlayPacket::ChatCommand(_) => {}
            C2sPlayPacket::Chat(p) => self.events.push(Event::ChatMessage {
                message: p.message.0,
                timestamp: Duration::from_millis(p.timestamp),
            }),
            C2sPlayPacket::ChatPreview(_) => {}
            C2sPlayPacket::ClientCommand(_) => {}
            C2sPlayPacket::ClientInformation(p) => {
                let old = self.settings.replace(Settings {
                    locale: p.locale.0,
                    view_distance: p.view_distance.0,
                    chat_mode: p.chat_mode,
                    chat_colors: p.chat_colors,
                    main_hand: p.main_hand,
                    displayed_skin_parts: p.displayed_skin_parts,
                    allow_server_listings: p.allow_server_listings,
                });

                self.events.push(Event::SettingsChanged(old));
            }
            C2sPlayPacket::CommandSuggestion(_) => {}
            C2sPlayPacket::ContainerButtonClick(_) => {}
            C2sPlayPacket::ContainerClose(_) => {}
            C2sPlayPacket::CustomPayload(_) => {}
            C2sPlayPacket::EditBook(_) => {}
            C2sPlayPacket::EntityTagQuery(_) => {}
            C2sPlayPacket::Interact(p) => {
                if let Some(id) = entities.get_with_network_id(p.entity_id.0) {
                    // TODO: verify that the client has line of sight to the targeted entity and
                    // that the distance is <=4 blocks.

                    self.events.push(Event::InteractWithEntity {
                        id,
                        sneaking: p.sneaking,
                        typ: match p.typ {
                            InteractType::Interact(hand) => InteractWithEntity::Interact(hand),
                            InteractType::Attack => InteractWithEntity::Attack,
                            InteractType::InteractAt((target, hand)) => {
                                InteractWithEntity::InteractAt { target, hand }
                            }
                        },
                    });
                }
            }
            C2sPlayPacket::JigsawGenerate(_) => {}
            C2sPlayPacket::KeepAlive(p) => {
                let last_keepalive_id = self.last_keepalive_id;
                if self.got_keepalive {
                    self.disconnect("Unexpected keepalive");
                } else if p.id != last_keepalive_id {
                    self.disconnect(format!(
                        "Keepalive ids don't match (expected {}, got {})",
                        last_keepalive_id, p.id
                    ));
                } else {
                    self.got_keepalive = true;
                }
            }
            C2sPlayPacket::LockDifficulty(_) => {}
            C2sPlayPacket::MovePlayerPosition(p) => {
                handle_movement_packet(self, false, p.position, self.yaw, self.pitch, p.on_ground)
            }
            C2sPlayPacket::MovePlayerPositionAndRotation(p) => {
                handle_movement_packet(self, false, p.position, p.yaw, p.pitch, p.on_ground)
            }
            C2sPlayPacket::MovePlayerRotation(p) => {
                handle_movement_packet(self, false, self.new_position, p.yaw, p.pitch, p.on_ground)
            }
            C2sPlayPacket::MovePlayerStatusOnly(p) => handle_movement_packet(
                self,
                false,
                self.new_position,
                self.yaw,
                self.pitch,
                p.on_ground,
            ),
            C2sPlayPacket::MoveVehicle(p) => {
                handle_movement_packet(self, true, p.position, p.yaw, p.pitch, self.on_ground);
            }
            C2sPlayPacket::PaddleBoat(p) => {
                self.events.push(Event::SteerBoat {
                    left_paddle_turning: p.left_paddle_turning,
                    right_paddle_turning: p.right_paddle_turning,
                });
            }
            C2sPlayPacket::PickItem(_) => {}
            C2sPlayPacket::PlaceRecipe(_) => {}
            C2sPlayPacket::PlayerAbilities(_) => {}
            C2sPlayPacket::PlayerAction(p) => {
                // TODO: verify dug block is within the correct distance from the client.
                // TODO: verify that the broken block is allowed to be broken?

                if p.sequence.0 != 0 {
                    self.dug_blocks.push(p.sequence.0);
                }

                self.events.push(match p.status {
                    DiggingStatus::StartedDigging => Event::Digging(Digging {
                        status: event::DiggingStatus::Start,
                        position: p.location,
                        face: p.face,
                    }),
                    DiggingStatus::CancelledDigging => Event::Digging(Digging {
                        status: event::DiggingStatus::Cancel,
                        position: p.location,
                        face: p.face,
                    }),
                    DiggingStatus::FinishedDigging => Event::Digging(Digging {
                        status: event::DiggingStatus::Finish,
                        position: p.location,
                        face: p.face,
                    }),
                    DiggingStatus::DropItemStack => return,
                    DiggingStatus::DropItem => return,
                    DiggingStatus::ShootArrowOrFinishEating => return,
                    DiggingStatus::SwapItemInHand => return,
                });
            }
            C2sPlayPacket::PlayerCommand(_) => {}
            C2sPlayPacket::PlayerInput(_) => {}
            C2sPlayPacket::Pong(_) => {}
            C2sPlayPacket::RecipeBookChangeSettings(_) => {}
            C2sPlayPacket::RecipeBookSeenRecipe(_) => {}
            C2sPlayPacket::RenameItem(_) => {}
            C2sPlayPacket::ResourcePack(_) => {}
            C2sPlayPacket::SeenAdvancements(_) => {}
            C2sPlayPacket::SelectTrade(_) => {}
            C2sPlayPacket::SetBeacon(_) => {}
            C2sPlayPacket::SetCarriedItem(_) => {}
            C2sPlayPacket::SetCommandBlock(_) => {}
            C2sPlayPacket::SetCommandBlockMinecart(_) => {}
            C2sPlayPacket::SetCreativeModeSlot(_) => {}
            C2sPlayPacket::SetJigsawBlock(_) => {}
            C2sPlayPacket::SetStructureBlock(_) => {}
            C2sPlayPacket::SignUpdate(_) => {}
            C2sPlayPacket::Swing(_) => {}
            C2sPlayPacket::TeleportToEntity(_) => {}
            C2sPlayPacket::UseItemOn(_) => {}
            C2sPlayPacket::UseItem(_) => {}
        }
    }

    pub(crate) fn update(
        &mut self,
        server: &Server,
        entities: &Entities,
        spatial_index: &SpatialIndex,
        chunks: &Chunks,
        meta: &WorldMeta,
    ) {
        // Mark the client as disconnected when appropriate.
        if self.recv.is_disconnected() || self.send.as_ref().map_or(true, |s| s.is_disconnected()) {
            self.send = None;
            return;
        }

        let current_tick = server.current_tick();

        // Send the join game packet and other initial packets. We defer this until now
        // so that the user can set the client's location, game mode, etc.
        if self.created_tick == current_tick {
            meta.player_list()
                .initial_packets(|pkt| self.send_packet(pkt));

            self.send_packet(Login {
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
                reduced_debug_info: false,
                enable_respawn_screen: false,
                is_debug: false,
                is_flat: meta.is_flat(),
                last_death_location: self
                    .death_location
                    .map(|(id, pos)| (ident!("{LIBRARY_NAMESPACE}:dimension_{}", id.0), pos)),
            });

            self.teleport(self.position(), self.yaw(), self.pitch());
        } else {
            if self.old_game_mode != self.new_game_mode {
                self.old_game_mode = self.new_game_mode;
                self.send_packet(GameEvent {
                    reason: GameEventReason::ChangeGameMode,
                    value: self.new_game_mode as i32 as f32,
                });
            }

            meta.player_list().packets(|pkt| self.send_packet(pkt));
        }

        // Update the players spawn position (compass position)
        if self.modified_spawn_position {
            self.modified_spawn_position = false;

            self.send_packet(SpawnPosition {
                location: self.spawn_position,
                angle: self.spawn_position_yaw,
            })
        }

        // Update view distance fog on the client if necessary.
        if self.old_max_view_distance != self.new_max_view_distance {
            self.old_max_view_distance = self.new_max_view_distance;
            if self.created_tick != current_tick {
                self.send_packet(SetChunkCacheRadius {
                    view_distance: BoundedInt(VarInt(self.new_max_view_distance as i32)),
                })
            }
        }

        // Check if it's time to send another keepalive.
        if current_tick % (server.tick_rate() * 8) == 0 {
            if self.got_keepalive {
                let id = rand::random();
                self.send_packet(KeepAlive { id });
                self.last_keepalive_id = id;
                self.got_keepalive = false;
            } else {
                self.disconnect("Timed out (no keepalive response)");
            }
        }

        let view_dist = self.view_distance();

        let center = ChunkPos::at(self.new_position.x, self.new_position.z);

        // Send the update view position packet if the client changes the chunk section
        // they're in.
        {
            let old_section = self.old_position.map(|n| (n / 16.0).floor() as i32);
            let new_section = self.new_position.map(|n| (n / 16.0).floor() as i32);

            if old_section != new_section {
                self.send_packet(SetChunkCacheCenter {
                    chunk_x: VarInt(new_section.x),
                    chunk_z: VarInt(new_section.z),
                })
            }
        }

        let dimension = server.dimension(meta.dimension());

        // Update existing chunks and unload those outside the view distance. Chunks
        // that have been overwritten also need to be unloaded.
        self.loaded_chunks.retain(|&pos| {
            // The cache stops chunk data packets from needing to be sent when a player
            // moves to an adjacent chunk and back to the original.
            let cache = 2;

            if let Some(chunk) = chunks.get(pos) {
                if is_chunk_in_view_distance(center, pos, view_dist + cache)
                    && chunk.created_tick() != current_tick
                {
                    chunk.block_change_packets(pos, dimension.min_y, |pkt| {
                        send_packet(&mut self.send, pkt)
                    });
                    return true;
                }
            }

            send_packet(
                &mut self.send,
                ForgetLevelChunk {
                    chunk_x: pos.x,
                    chunk_z: pos.z,
                },
            );
            false
        });

        // Load new chunks within the view distance
        for pos in chunks_in_view_distance(center, view_dist) {
            if let Some(chunk) = chunks.get(pos) {
                if self.loaded_chunks.insert(pos) {
                    self.send_packet(chunk.chunk_data_packet(pos));
                    chunk.block_change_packets(pos, dimension.min_y, |pkt| self.send_packet(pkt));
                }
            }
        }

        // Acknowledge broken blocks.
        for seq in self.dug_blocks.drain(..) {
            send_packet(
                &mut self.send,
                BlockChangeAck {
                    sequence: VarInt(seq),
                },
            )
        }

        // This is done after the chunks are loaded so that the "downloading terrain"
        // screen is closed at the appropriate time.
        if self.teleported_this_tick {
            self.teleported_this_tick = false;

            self.send_packet(PlayerPosition {
                position: self.new_position,
                yaw: self.yaw,
                pitch: self.pitch,
                flags: PlayerPositionFlags::new(false, false, false, false, false),
                teleport_id: VarInt((self.teleport_id_counter - 1) as i32),
                dismount_vehicle: false,
            });
        }

        for msg in self.msgs_to_send.drain(..) {
            send_packet(
                &mut self.send,
                SystemChat {
                    chat: msg,
                    typ: VarInt(0),
                },
            );
        }

        let mut entities_to_unload = Vec::new();

        // Update all entities that are visible and unload entities that are no
        // longer visible.
        self.loaded_entities.retain(|&id| {
            if let Some(entity) = entities.get(id) {
                debug_assert!(entity.typ() != EntityType::Marker);
                if self.new_position.distance(entity.position()) <= view_dist as f64 * 16.0
                    && !entity.flags().type_modified()
                {
                    if let Some(meta) = entity.updated_metadata_packet(id) {
                        send_packet(&mut self.send, meta);
                    }

                    let position_delta = entity.position() - entity.old_position();
                    let needs_teleport = position_delta.map(f64::abs).reduce_partial_max() >= 8.0;
                    let flags = entity.flags();

                    if entity.position() != entity.old_position()
                        && !needs_teleport
                        && flags.yaw_or_pitch_modified()
                    {
                        send_packet(
                            &mut self.send,
                            MoveEntityPositionAndRotation {
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
                                &mut self.send,
                                MoveEntityPosition {
                                    entity_id: VarInt(id.to_network_id()),
                                    delta: (position_delta * 4096.0).as_(),
                                    on_ground: entity.on_ground(),
                                },
                            );
                        }

                        if flags.yaw_or_pitch_modified() {
                            send_packet(
                                &mut self.send,
                                MoveEntityRotation {
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
                            &mut self.send,
                            TeleportEntity {
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
                            &mut self.send,
                            SetEntityMotion {
                                entity_id: VarInt(id.to_network_id()),
                                velocity: velocity_to_packet_units(entity.velocity()),
                            },
                        );
                    }

                    if flags.head_yaw_modified() {
                        send_packet(
                            &mut self.send,
                            RotateHead {
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
            self.send_packet(RemoveEntities {
                entities: entities_to_unload,
            });
        }

        // Update the client's own player metadata.
        let mut data = Vec::new();
        self.player_meta.updated_metadata(&mut data);

        if !data.is_empty() {
            data.push(0xff);

            self.send_packet(SetEntityMetadata {
                entity_id: VarInt(0),
                metadata: RawBytes(data),
            });
        }
        self.player_meta.clear_modifications();

        // Spawn new entities within the view distance.
        let pos = self.position();
        spatial_index.query::<_, _, ()>(
            |bb| bb.projected_point(pos).distance(pos) <= view_dist as f64 * 16.0,
            |id, _| {
                if self.loaded_entities.insert(id) {
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

        self.old_position = self.new_position;
    }
}

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
            value: vec![ChatTypeRegistryEntry {
                name: ident!("system"),
                id: 0,
                element: ChatType {
                    chat: ChatTypeChat {},
                    narration: ChatTypeNarration {
                        priority: "system".to_string(),
                    },
                },
            }],
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
