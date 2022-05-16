use std::collections::HashSet;
use std::iter::FusedIterator;
use std::ops::Deref;

use flume::{Receiver, Sender, TrySendError};
use rayon::iter::ParallelIterator;
use uuid::Uuid;
use vek::Vec3;

use crate::block_pos::BlockPos;
use crate::config::{
    Biome, BiomeGrassColorModifier, BiomePrecipitation, Dimension, DimensionEffects, DimensionId,
};
use crate::entity::EntityType;
pub use crate::packets::play::GameMode;
use crate::packets::play::{
    Biome as BiomeRegistryBiome, BiomeAdditionsSound, BiomeEffects, BiomeMoodSound, BiomeMusic,
    BiomeParticle, BiomeParticleOptions, BiomeProperty, BiomeRegistry, ChangeGameState,
    ChangeGameStateReason, ClientPlayPacket, DestroyEntities, DimensionCodec, DimensionType,
    DimensionTypeRegistry, DimensionTypeRegistryEntry, Disconnect, JoinGame, KeepAliveClientbound,
    PlayerPositionAndLook, PlayerPositionAndLookFlags, ServerPlayPacket, SpawnPosition,
    UnloadChunk, UpdateViewDistance, UpdateViewPosition,
};
use crate::protocol::{BoundedInt, Nbt};
use crate::server::ServerPacketChannels;
use crate::slotmap::{Key, SlotMap};
use crate::util::{chunks_in_view_distance, is_chunk_in_view_distance};
use crate::var_int::VarInt;
use crate::{ident, ChunkPos, Chunks, Entities, EntityId, Server, Text, Ticks, LIBRARY_NAMESPACE};

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
        self.sm.count()
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

    pub(crate) fn create(&mut self, client: Client) -> ClientId {
        ClientId(self.0.sm.insert(client))
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
    send: Option<Sender<ClientPlayPacket>>,
    recv: Receiver<ServerPlayPacket>,
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
        packet_channels: ServerPacketChannels,
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

    /// Changes the point at which compasses point at.
    pub fn set_spawn_position(&mut self, pos: impl Into<BlockPos>, yaw_degrees: f32) {
        let pos = pos.into();
        if pos != self.0.spawn_position || yaw_degrees != self.0.spawn_position_yaw {
            self.0.spawn_position = pos;
            self.0.spawn_position_yaw = yaw_degrees;
            self.0.modified_spawn_position = true;
        }
    }

    /// Attempts to enqueue a play packet to be sent to this client. The client
    /// is disconnected if the clientbound packet buffer is full.
    pub(crate) fn send_packet(&mut self, packet: impl Into<ClientPlayPacket>) {
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
        chunks: &Chunks,
        dimension_id: DimensionId,
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

        let dimension = server.dimension(dimension_id);

        // Send the join game packet and other initial packets. We defer this until now
        // so that the user can set the client's location, game mode, etc.
        if self.created_tick == server.current_tick() {
            self.send_packet(JoinGame {
                entity_id: 0,       // EntityId 0 is reserved for clients.
                is_hardcore: false, // TODO
                gamemode: self.new_game_mode,
                previous_gamemode: self.old_game_mode,
                dimension_names: server
                    .dimensions()
                    .map(|(id, _)| ident!("{LIBRARY_NAMESPACE}:dimension_{}", id.0))
                    .collect(),
                dimension_codec: Nbt(make_dimension_codec(server)),
                dimension: Nbt(to_dimension_registry_item(dimension)),
                dimension_name: ident!("{LIBRARY_NAMESPACE}:dimension_{}", dimension_id.0),
                hashed_seed: 0,
                max_players: VarInt(0),
                view_distance: BoundedInt(VarInt(self.new_max_view_distance as i32)),
                simulation_distance: VarInt(16),
                reduced_debug_info: false,
                enable_respawn_screen: false, // TODO
                is_debug: false,
                is_flat: false, // TODO
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
            if self.0.created_tick != server.current_tick() {
                self.send_packet(UpdateViewDistance {
                    view_distance: BoundedInt(VarInt(self.0.new_max_view_distance as i32)),
                })
            }
        }

        // Check if it's time to send another keepalive.
        if server.current_tick() % (server.tick_rate() * 8) == 0 {
            if self.0.got_keepalive {
                let id = rand::random();
                self.send_packet(KeepAliveClientbound { id });
                self.0.last_keepalive_id = id;
                self.0.got_keepalive = false;
            } else {
                self.disconnect("Timed out (no keepalive response)");
            }
        }

        // The actual view distance.
        let view_dist = self
            .0
            .settings
            .as_ref()
            .map_or(2, |s| s.view_distance)
            .min(self.new_max_view_distance);

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

        // Update existing chunks and unload those outside the view distance. Chunks
        // that have been overwritten also need to be unloaded.
        self.0.loaded_chunks.retain(|&pos| {
            // The cache stops chunk data packets from needing to be sent when a player
            // moves to an adjacent chunk and back to the original.
            let cache = 2;

            if let Some(chunk) = chunks.get(pos) {
                if is_chunk_in_view_distance(center, pos, view_dist + cache)
                    && chunk.created_tick() != server.current_tick()
                {
                    if let Some(pkt) = chunk.block_change_packet(pos) {
                        send_packet(&mut self.0.send, pkt);
                    }
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
                    if let Some(pkt) = chunk.block_change_packet(pos) {
                        self.send_packet(pkt);
                    }
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
                if self.0.new_position.distance(entity.position()) <= view_dist as f64 * 16.0 {
                    todo!("update entity");
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
        for (id, entity) in entities.iter() {
            if self.position().distance(entity.position()) <= view_dist as f64 * 16.0
                && entity.typ() != EntityType::Marker
                && self.0.loaded_entities.insert(id)
            {
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

        self.0.old_position = self.0.new_position;
    }

    fn handle_serverbound_packet(&mut self, pkt: ServerPlayPacket) {
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
            ServerPlayPacket::TeleportConfirm(p) => {
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
            ServerPlayPacket::QueryBlockNbt(_) => {}
            ServerPlayPacket::SetDifficulty(_) => {}
            ServerPlayPacket::ChatMessageServerbound(_) => {}
            ServerPlayPacket::ClientStatus(_) => {}
            ServerPlayPacket::ClientSettings(p) => {
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
            ServerPlayPacket::TabCompleteServerbound(_) => {}
            ServerPlayPacket::ClickWindowButton(_) => {}
            ServerPlayPacket::ClickWindow(_) => {}
            ServerPlayPacket::CloseWindow(_) => {}
            ServerPlayPacket::PluginMessageServerbound(_) => {}
            ServerPlayPacket::EditBook(_) => {}
            ServerPlayPacket::QueryEntityNbt(_) => {}
            ServerPlayPacket::InteractEntity(_) => {}
            ServerPlayPacket::GenerateStructure(_) => {}
            ServerPlayPacket::KeepAliveServerbound(p) => {
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
            ServerPlayPacket::LockDifficulty(_) => {}
            ServerPlayPacket::PlayerPosition(p) => {
                handle_movement_packet(client, p.position, client.yaw, client.pitch, p.on_ground)
            }
            ServerPlayPacket::PlayerPositionAndRotation(p) => {
                handle_movement_packet(client, p.position, p.yaw, p.pitch, p.on_ground)
            }
            ServerPlayPacket::PlayerRotation(p) => {
                handle_movement_packet(client, client.new_position, p.yaw, p.pitch, p.on_ground)
            }

            ServerPlayPacket::PlayerMovement(p) => handle_movement_packet(
                client,
                client.new_position,
                client.yaw,
                client.pitch,
                p.on_ground,
            ),
            ServerPlayPacket::VehicleMoveServerbound(_) => {}
            ServerPlayPacket::SteerBoat(_) => {}
            ServerPlayPacket::PickItem(_) => {}
            ServerPlayPacket::CraftRecipeRequest(_) => {}
            ServerPlayPacket::PlayerAbilitiesServerbound(_) => {}
            ServerPlayPacket::PlayerDigging(_) => {}
            ServerPlayPacket::EntityAction(_) => {}
            ServerPlayPacket::SteerVehicle(_) => {}
            ServerPlayPacket::Pong(_) => {}
            ServerPlayPacket::SetRecipeBookState(_) => {}
            ServerPlayPacket::SetDisplayedRecipe(_) => {}
            ServerPlayPacket::NameItem(_) => {}
            ServerPlayPacket::ResourcePackStatus(_) => {}
            ServerPlayPacket::AdvancementTab(_) => {}
            ServerPlayPacket::SelectTrade(_) => {}
            ServerPlayPacket::SetBeaconEffect(_) => {}
            ServerPlayPacket::HeldItemChangeServerbound(_) => {}
            ServerPlayPacket::UpdateCommandBlock(_) => {}
            ServerPlayPacket::UpdateCommandBlockMinecart(_) => {}
            ServerPlayPacket::CreativeInventoryAction(_) => {}
            ServerPlayPacket::UpdateJigsawBlock(_) => {}
            ServerPlayPacket::UpdateStructureBlock(_) => {}
            ServerPlayPacket::UpdateSign(_) => {}
            ServerPlayPacket::PlayerArmSwing(_) => {}
            ServerPlayPacket::Spectate(_) => {}
            ServerPlayPacket::PlayerBlockPlacement(_) => {}
            ServerPlayPacket::UseItem(_) => {}
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

pub use crate::packets::play::{ChatMode, DisplayedSkinParts, MainHand};

fn send_packet(send_opt: &mut Option<Sender<ClientPlayPacket>>, pkt: impl Into<ClientPlayPacket>) {
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

fn make_dimension_codec(server: &Server) -> DimensionCodec {
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

    DimensionCodec {
        dimension_type_registry: DimensionTypeRegistry {
            typ: ident!("dimension_type"),
            value: dims,
        },
        biome_registry: BiomeRegistry {
            typ: ident!("worldgen/biome"),
            value: biomes,
        },
    }
}

fn to_dimension_registry_item(dim: &Dimension) -> DimensionType {
    DimensionType {
        piglin_safe: true,
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
        has_raids: true,
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
