/// Contains the [`Event`] enum and related data types.
mod event;
use std::collections::{HashSet, VecDeque};
use std::iter::FusedIterator;
use std::time::Duration;

use bitfield_struct::bitfield;
pub use event::*;
use flume::{Receiver, Sender, TrySendError};
use rayon::iter::ParallelIterator;
use uuid::Uuid;
use vek::Vec3;

use crate::biome::{Biome, BiomeGrassColorModifier, BiomePrecipitation};
use crate::dimension::{Dimension, DimensionEffects};
use crate::entity::data::Player;
use crate::entity::{velocity_to_packet_units, EntityKind};
use crate::player_textures::SignedPlayerTextures;
use crate::protocol::packets::play::c2s::{
    C2sPlayPacket, DiggingStatus, InteractKind, PlayerCommandId,
};
pub use crate::protocol::packets::play::s2c::SetTitleAnimationTimes as TitleAnimationTimes;
use crate::protocol::packets::play::s2c::{
    Animate, Biome as BiomeRegistryBiome, BiomeAdditionsSound, BiomeEffects, BiomeMoodSound,
    BiomeMusic, BiomeParticle, BiomeParticleOptions, BiomeProperty, BiomeRegistry, BlockChangeAck,
    ChatType, ChatTypeChat, ChatTypeNarration, ChatTypeRegistry, ChatTypeRegistryEntry,
    ClearTitles, DimensionType, DimensionTypeRegistry, DimensionTypeRegistryEntry, Disconnect,
    EntityEvent, ForgetLevelChunk, GameEvent, GameEventReason, KeepAlive, Login,
    MoveEntityPosition, MoveEntityPositionAndRotation, MoveEntityRotation, PlayerPosition,
    PlayerPositionFlags, RegistryCodec, RemoveEntities, Respawn, RotateHead, S2cPlayPacket,
    SetChunkCacheCenter, SetChunkCacheRadius, SetEntityMetadata, SetEntityMotion, SetSubtitleText,
    SetTitleText, SpawnPosition, SystemChat, TeleportEntity, ENTITY_EVENT_MAX_BOUND,
};
use crate::protocol::{BoundedInt, ByteAngle, Nbt, RawBytes, VarInt};
use crate::server::C2sPacketChannels;
use crate::slotmap::{Key, SlotMap};
use crate::util::{chunks_in_view_distance, is_chunk_in_view_distance};
use crate::{
    ident, BlockPos, ChunkPos, DimensionId, Entities, Entity, EntityId, NewClientData,
    SharedServer, Text, Ticks, WorldId, Worlds, LIBRARY_NAMESPACE,
};

pub struct Clients {
    sm: SlotMap<Client>,
}

impl Clients {
    pub(crate) fn new() -> Self {
        Self { sm: SlotMap::new() }
    }

    pub(crate) fn insert(&mut self, client: Client) -> (ClientId, &mut Client) {
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

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct ClientId(Key);

impl ClientId {
    pub const NULL: Self = Self(Key::NULL);
}

/// Represents a client connected to the server after logging in.
pub struct Client {
    /// Setting this to `None` disconnects the client.
    send: SendOpt,
    recv: Receiver<C2sPlayPacket>,
    /// The tick this client was created.
    created_tick: Ticks,
    uuid: Uuid,
    username: String,
    textures: Option<SignedPlayerTextures>,
    world: WorldId,
    new_position: Vec3<f64>,
    old_position: Vec3<f64>,
    /// Measured in degrees
    yaw: f32,
    /// Measured in degrees
    pitch: f32,
    /// Counts up as teleports are made.
    teleport_id_counter: u32,
    /// The number of pending client teleports that have yet to receive a
    /// confirmation. Inbound client position packets are ignored while this
    /// is nonzero.
    pending_teleports: u32,
    spawn_position: BlockPos,
    spawn_position_yaw: f32,
    death_location: Option<(DimensionId, BlockPos)>,
    events: VecDeque<ClientEvent>,
    /// The ID of the last keepalive sent.
    last_keepalive_id: i64,
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
    flags: ClientFlags,
    /// The data for the client's own player entity.
    player_data: Player,
}

#[bitfield(u16)]
pub(crate) struct ClientFlags {
    spawn: bool,
    sneaking: bool,
    sprinting: bool,
    jumping_with_horse: bool,
    on_ground: bool,
    /// If any of position, yaw, or pitch were modified by the
    /// user this tick.
    teleported_this_tick: bool,
    /// If spawn_position or spawn_position_yaw were modified this tick.
    modified_spawn_position: bool,
    /// If the last sent keepalive got a response.
    got_keepalive: bool,
    hardcore: bool,
    #[bits(7)]
    _pad: u8,
}

impl Client {
    pub(crate) fn new(
        packet_channels: C2sPacketChannels,
        server: &SharedServer,
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
            world: WorldId::default(),
            new_position: Vec3::default(),
            old_position: Vec3::default(),
            yaw: 0.0,
            pitch: 0.0,
            teleport_id_counter: 0,
            pending_teleports: 0,
            spawn_position: BlockPos::default(),
            spawn_position_yaw: 0.0,
            death_location: None,
            events: VecDeque::new(),
            last_keepalive_id: 0,
            new_max_view_distance: 16,
            old_max_view_distance: 0,
            loaded_entities: HashSet::new(),
            loaded_chunks: HashSet::new(),
            new_game_mode: GameMode::Survival,
            old_game_mode: GameMode::Survival,
            settings: None,
            dug_blocks: Vec::new(),
            msgs_to_send: Vec::new(),
            flags: ClientFlags::new()
                .with_modified_spawn_position(true)
                .with_got_keepalive(true),
            player_data: Player::new(),
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

    pub fn world(&self) -> WorldId {
        self.world
    }

    pub fn spawn(&mut self, world: WorldId) {
        self.world = world;
        self.flags.set_spawn(true);
    }

    /// Sends a system message to the player.
    pub fn send_message(&mut self, msg: impl Into<Text>) {
        // We buffer messages because weird things happen if we send them before the
        // login packet.
        self.msgs_to_send.push(msg.into());
    }

    pub fn position(&self) -> Vec3<f64> {
        self.new_position
    }

    pub fn teleport(&mut self, pos: impl Into<Vec3<f64>>, yaw: f32, pitch: f32) {
        self.new_position = pos.into();

        self.yaw = yaw;
        self.pitch = pitch;

        if !self.flags.teleported_this_tick() {
            self.flags.set_teleported_this_tick(true);

            self.pending_teleports = match self.pending_teleports.checked_add(1) {
                Some(n) => n,
                None => {
                    log::warn!("too many pending teleports for {}", self.username());
                    self.disconnect_no_reason();
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
            self.flags.set_modified_spawn_position(true);
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

    pub fn set_title(
        &mut self,
        title: impl Into<Text>,
        subtitle: impl Into<Text>,
        animation: impl Into<Option<TitleAnimationTimes>>,
    ) {
        let title = title.into();
        let subtitle = subtitle.into();

        self.send_packet(SetTitleText { text: title });

        if !subtitle.is_empty() {
            self.send_packet(SetSubtitleText {
                subtitle_text: subtitle,
            });
        }

        if let Some(anim) = animation.into() {
            self.send_packet(anim);
        }
    }

    pub fn clear_title(&mut self) {
        self.send_packet(ClearTitles { reset: true });
    }

    pub fn on_ground(&self) -> bool {
        self.flags.on_ground()
    }

    pub fn is_disconnected(&self) -> bool {
        self.send.is_none()
    }

    pub fn pop_event(&mut self) -> Option<ClientEvent> {
        self.events.pop_front()
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

    /// Must be set on the same tick the client joins the game.
    pub fn set_hardcore(&mut self, hardcore: bool) {
        self.flags.set_hardcore(hardcore);
    }

    pub fn is_hardcore(&mut self) -> bool {
        self.flags.hardcore()
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
            log::info!("disconnecting client '{}'", self.username);
            self.send = None;
        }
    }

    pub fn data(&self) -> &Player {
        &self.player_data
    }

    pub fn data_mut(&mut self) -> &mut Player {
        &mut self.player_data
    }

    /// Attempts to enqueue a play packet to be sent to this client. The client
    /// is disconnected if the clientbound packet buffer is full.
    #[cfg(feature = "protocol")]
    pub fn send_packet(&mut self, packet: impl Into<S2cPlayPacket>) {
        send_packet(&mut self.send, packet);
    }

    #[cfg(not(feature = "protocol"))]
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
                let event = ClientEvent::Movement {
                    position: client.new_position,
                    yaw: client.yaw,
                    pitch: client.pitch,
                    on_ground: client.flags.on_ground(),
                };

                client.new_position = new_position;
                client.yaw = new_yaw;
                client.pitch = new_pitch;
                client.flags.set_on_ground(new_on_ground);

                client.events.push_back(event);
            }
        }

        match pkt {
            C2sPlayPacket::AcceptTeleportation(p) => {
                if self.pending_teleports == 0 {
                    log::warn!("unexpected teleport confirmation from {}", self.username());
                    self.disconnect_no_reason();
                    return;
                }

                let got = p.teleport_id.0 as u32;
                let expected = self
                    .teleport_id_counter
                    .wrapping_sub(self.pending_teleports);

                if got == expected {
                    self.pending_teleports -= 1;
                } else {
                    log::warn!(
                        "unexpected teleport ID from {} (expected {expected}, got {got})",
                        self.username()
                    );
                    self.disconnect_no_reason();
                    return;
                }
            }
            C2sPlayPacket::BlockEntityTagQuery(_) => {}
            C2sPlayPacket::ChangeDifficulty(_) => {}
            C2sPlayPacket::ChatCommand(_) => {}
            C2sPlayPacket::Chat(p) => self.events.push_back(ClientEvent::ChatMessage {
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

                self.events.push_back(ClientEvent::SettingsChanged(old));
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

                    self.events.push_back(ClientEvent::InteractWithEntity {
                        id,
                        sneaking: p.sneaking,
                        kind: match p.kind {
                            InteractKind::Interact(hand) => InteractWithEntity::Interact(hand),
                            InteractKind::Attack => InteractWithEntity::Attack,
                            InteractKind::InteractAt((target, hand)) => {
                                InteractWithEntity::InteractAt { target, hand }
                            }
                        },
                    });
                }
            }
            C2sPlayPacket::JigsawGenerate(_) => {}
            C2sPlayPacket::KeepAlive(p) => {
                let last_keepalive_id = self.last_keepalive_id;
                if self.flags.got_keepalive() {
                    log::warn!("unexpected keepalive from player {}", self.username());
                    self.disconnect_no_reason();
                } else if p.id != last_keepalive_id {
                    log::warn!(
                        "keepalive ids for player {} don't match (expected {}, got {})",
                        self.username(),
                        last_keepalive_id,
                        p.id
                    );
                    self.disconnect_no_reason();
                } else {
                    self.flags.set_got_keepalive(true);
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
                handle_movement_packet(
                    self,
                    true,
                    p.position,
                    p.yaw,
                    p.pitch,
                    self.flags.on_ground(),
                );
            }
            C2sPlayPacket::PaddleBoat(p) => {
                self.events.push_back(ClientEvent::SteerBoat {
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

                self.events.push_back(match p.status {
                    DiggingStatus::StartedDigging => ClientEvent::Digging(Digging {
                        status: event::DiggingStatus::Start,
                        position: p.location,
                        face: p.face,
                    }),
                    DiggingStatus::CancelledDigging => ClientEvent::Digging(Digging {
                        status: event::DiggingStatus::Cancel,
                        position: p.location,
                        face: p.face,
                    }),
                    DiggingStatus::FinishedDigging => ClientEvent::Digging(Digging {
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
            C2sPlayPacket::PlayerCommand(e) => {
                // TODO: validate:
                // - Can't sprint and sneak at the same time
                // - Can't leave bed while not in a bed.
                // - Can't jump with a horse if not on a horse
                // - Can't open horse inventory if not on a horse.
                // - Can't fly with elytra if not wearing an elytra.
                self.events.push_back(match e.action_id {
                    PlayerCommandId::StartSneaking => {
                        self.flags.set_sneaking(true);
                        ClientEvent::StartSneaking
                    }
                    PlayerCommandId::StopSneaking => {
                        self.flags.set_sneaking(false);
                        ClientEvent::StopSneaking
                    }
                    PlayerCommandId::LeaveBed => ClientEvent::LeaveBed,
                    PlayerCommandId::StartSprinting => {
                        self.flags.set_sprinting(true);
                        ClientEvent::StartSprinting
                    }
                    PlayerCommandId::StopSprinting => {
                        self.flags.set_sprinting(false);
                        ClientEvent::StopSprinting
                    }
                    PlayerCommandId::StartJumpWithHorse => {
                        self.flags.set_jumping_with_horse(true);
                        ClientEvent::StartJumpWithHorse(e.jump_boost.0 .0 as u8)
                    }
                    PlayerCommandId::StopJumpWithHorse => {
                        self.flags.set_jumping_with_horse(false);
                        ClientEvent::StopJumpWithHorse
                    }
                    PlayerCommandId::OpenHorseInventory => ClientEvent::OpenHorseInventory,
                    PlayerCommandId::StartFlyingWithElytra => ClientEvent::StartFlyingWithElytra,
                });
            }
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
            C2sPlayPacket::Swing(p) => self.events.push_back(ClientEvent::ArmSwing(p.hand)),
            C2sPlayPacket::TeleportToEntity(_) => {}
            C2sPlayPacket::UseItemOn(_) => {}
            C2sPlayPacket::UseItem(_) => {}
        }
    }

    pub(crate) fn update(&mut self, shared: &SharedServer, entities: &Entities, worlds: &Worlds) {
        // Mark the client as disconnected when appropriate.
        if self.recv.is_disconnected() || self.send.as_ref().map_or(true, |s| s.is_disconnected()) {
            self.send = None;
            return;
        }

        let world = match worlds.get(self.world) {
            Some(world) => world,
            None => {
                log::warn!(
                    "client {} is in an invalid world and must be disconnected",
                    self.username()
                );
                self.disconnect_no_reason();
                return;
            }
        };

        let current_tick = shared.current_tick();

        // Send the join game packet and other initial packets. We defer this until now
        // so that the user can set the client's location, game mode, etc.
        if self.created_tick == current_tick {
            world
                .meta
                .player_list()
                .initial_packets(|pkt| self.send_packet(pkt));

            let mut dimension_names: Vec<_> = shared
                .dimensions()
                .map(|(id, _)| ident!("{LIBRARY_NAMESPACE}:dimension_{}", id.0))
                .collect();

            dimension_names.push(ident!("{LIBRARY_NAMESPACE}:dummy_dimension"));

            self.send_packet(Login {
                entity_id: 0, // EntityId 0 is reserved for clients.
                is_hardcore: self.flags.hardcore(),
                gamemode: self.new_game_mode,
                previous_gamemode: self.old_game_mode,
                dimension_names,
                registry_codec: Nbt(make_dimension_codec(shared)),
                dimension_type_name: ident!(
                    "{LIBRARY_NAMESPACE}:dimension_type_{}",
                    world.meta.dimension().0
                ),
                dimension_name: ident!(
                    "{LIBRARY_NAMESPACE}:dimension_{}",
                    world.meta.dimension().0
                ),
                hashed_seed: 0,
                max_players: VarInt(0),
                view_distance: BoundedInt(VarInt(self.new_max_view_distance as i32)),
                simulation_distance: VarInt(16),
                reduced_debug_info: false,
                enable_respawn_screen: false,
                is_debug: false,
                is_flat: world.meta.is_flat(),
                last_death_location: self
                    .death_location
                    .map(|(id, pos)| (ident!("{LIBRARY_NAMESPACE}:dimension_{}", id.0), pos)),
            });

            self.teleport(self.position(), self.yaw(), self.pitch());
        } else {
            if self.flags.spawn() {
                self.loaded_entities.clear();
                self.loaded_chunks.clear();

                // TODO: clear player list.

                // Client bug workaround: send the client to a dummy dimension first.
                self.send_packet(Respawn {
                    dimension_type_name: ident!("{LIBRARY_NAMESPACE}:dimension_type_0"),
                    dimension_name: ident!("{LIBRARY_NAMESPACE}:dummy_dimension"),
                    hashed_seed: 0,
                    game_mode: self.game_mode(),
                    previous_game_mode: self.game_mode(),
                    is_debug: false,
                    is_flat: false,
                    copy_metadata: true,
                    last_death_location: None,
                });

                self.send_packet(Respawn {
                    dimension_type_name: ident!(
                        "{LIBRARY_NAMESPACE}:dimension_type_{}",
                        world.meta.dimension().0
                    ),
                    dimension_name: ident!(
                        "{LIBRARY_NAMESPACE}:dimension_{}",
                        world.meta.dimension().0
                    ),
                    hashed_seed: 0,
                    game_mode: self.game_mode(),
                    previous_game_mode: self.game_mode(),
                    is_debug: false,
                    is_flat: world.meta.is_flat(),
                    copy_metadata: true,
                    last_death_location: self
                        .death_location
                        .map(|(id, pos)| (ident!("{LIBRARY_NAMESPACE}:dimension_{}", id.0), pos)),
                });

                self.teleport(self.position(), self.yaw(), self.pitch());
            }

            if self.old_game_mode != self.new_game_mode {
                self.old_game_mode = self.new_game_mode;
                self.send_packet(GameEvent {
                    reason: GameEventReason::ChangeGameMode,
                    value: self.new_game_mode as i32 as f32,
                });
            }

            world
                .meta
                .player_list()
                .diff_packets(|pkt| self.send_packet(pkt));
        }

        // Update the players spawn position (compass position)
        if self.flags.modified_spawn_position() {
            self.flags.set_modified_spawn_position(false);

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
        if current_tick % (shared.tick_rate() * 8) == 0 {
            if self.flags.got_keepalive() {
                let id = rand::random();
                self.send_packet(KeepAlive { id });
                self.last_keepalive_id = id;
                self.flags.set_got_keepalive(false);
            } else {
                log::warn!(
                    "player {} timed out (no keepalive response)",
                    self.username()
                );
                self.disconnect_no_reason();
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

        let dimension = shared.dimension(world.meta.dimension());

        // Update existing chunks and unload those outside the view distance. Chunks
        // that have been overwritten also need to be unloaded.
        self.loaded_chunks.retain(|&pos| {
            // The cache stops chunk data packets from needing to be sent when a player
            // moves to an adjacent chunk and back to the original.
            let cache = 2;

            if let Some(chunk) = world.chunks.get(pos) {
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
            if let Some(chunk) = world.chunks.get(pos) {
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
        if self.flags.teleported_this_tick() {
            self.flags.set_teleported_this_tick(false);

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
                    kind: VarInt(0),
                },
            );
        }

        let mut entities_to_unload = Vec::new();

        // Update all entities that are visible and unload entities that are no
        // longer visible.
        self.loaded_entities.retain(|&id| {
            if let Some(entity) = entities.get(id) {
                debug_assert!(entity.kind() != EntityKind::Marker);
                if self.new_position.distance(entity.position()) <= view_dist as f64 * 16.0 {
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

                    send_entity_events(&mut self.send, id, entity);

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
        self.player_data.updated_metadata(&mut data);

        if !data.is_empty() {
            data.push(0xff);

            self.send_packet(SetEntityMetadata {
                entity_id: VarInt(0),
                metadata: RawBytes(data),
            });
        }
        self.player_data.clear_modifications();

        // Spawn new entities within the view distance.
        let pos = self.position();
        world.spatial_index.query::<_, _, ()>(
            |bb| bb.projected_point(pos).distance(pos) <= view_dist as f64 * 16.0,
            |id, _| {
                let entity = entities
                    .get(id)
                    .expect("entities in spatial index should be valid");
                if entity.kind() != EntityKind::Marker
                    && entity.uuid() != self.uuid
                    && self.loaded_entities.insert(id)
                {
                    self.send_packet(
                        entity
                            .spawn_packet(id)
                            .expect("should not be a marker entity"),
                    );

                    if let Some(meta) = entity.initial_metadata_packet(id) {
                        self.send_packet(meta);
                    }

                    send_entity_events(&mut self.send, id, entity);
                }
                None
            },
        );

        for &code in self.player_data.event_codes() {
            if code <= ENTITY_EVENT_MAX_BOUND as u8 {
                send_packet(
                    &mut self.send,
                    EntityEvent {
                        entity_id: 0,
                        entity_status: BoundedInt(code),
                    },
                );
            }
            // Don't bother sending animations since it shouldn't be visible to
            // the client.
        }

        self.old_position = self.new_position;
        self.flags.set_spawn(false);
    }
}

type SendOpt = Option<Sender<S2cPlayPacket>>;

fn send_packet(send_opt: &mut SendOpt, pkt: impl Into<S2cPlayPacket>) {
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

fn send_entity_events(send_opt: &mut SendOpt, id: EntityId, entity: &Entity) {
    for &code in entity.data().event_codes() {
        if code <= ENTITY_EVENT_MAX_BOUND as u8 {
            send_packet(
                send_opt,
                EntityEvent {
                    entity_id: id.to_network_id(),
                    entity_status: BoundedInt(code),
                },
            );
        } else {
            send_packet(
                send_opt,
                Animate {
                    entity_id: VarInt(id.to_network_id()),
                    animation: BoundedInt(code - ENTITY_EVENT_MAX_BOUND as u8 - 1),
                },
            )
        }
    }
}

fn make_dimension_codec(shared: &SharedServer) -> RegistryCodec {
    let mut dims = Vec::new();
    for (id, dim) in shared.dimensions() {
        let id = id.0 as i32;
        dims.push(DimensionTypeRegistryEntry {
            name: ident!("{LIBRARY_NAMESPACE}:dimension_type_{id}"),
            id,
            element: to_dimension_registry_item(dim),
        })
    }

    let mut biomes = Vec::new();
    for (id, biome) in shared.biomes() {
        biomes.push(to_biome_registry_item(biome, id.0 as i32));
    }

    // The client needs a biome named "minecraft:plains" in the registry to
    // connect. This is probably a bug.
    //
    // If the issue is resolved, just delete this block.
    if !biomes.iter().any(|b| b.name == ident!("plains")) {
        let biome = Biome::default();
        assert_eq!(biome.name, ident!("plains"));
        biomes.push(to_biome_registry_item(&biome, biomes.len() as i32));
    }

    RegistryCodec {
        dimension_type_registry: DimensionTypeRegistry {
            kind: ident!("dimension_type"),
            value: dims,
        },
        biome_registry: BiomeRegistry {
            kind: ident!("worldgen/biome"),
            value: biomes,
        },
        chat_type_registry: ChatTypeRegistry {
            kind: ident!("chat_type"),
            value: vec![ChatTypeRegistryEntry {
                name: ident!("system"),
                id: 0,
                element: ChatType {
                    chat: ChatTypeChat {},
                    narration: ChatTypeNarration {
                        priority: "system".to_owned(),
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
                grass_color: biome.grass_color.map(|x| x as i32),
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
                options: BiomeParticleOptions {
                    kind: p.kind.clone(),
                },
            }),
        },
    }
}
