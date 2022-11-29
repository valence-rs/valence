//! Connections to the server after logging in.

use std::collections::HashSet;
use std::iter::FusedIterator;
use std::net::IpAddr;
use std::num::Wrapping;
use std::ops::{Deref, DerefMut};
use std::{array, mem};

use anyhow::{bail, Context};
pub use bitfield_struct::bitfield;
pub use event::ClientEvent;
use rayon::iter::ParallelIterator;
use tokio::sync::OwnedSemaphorePermit;
use tracing::{info, warn};
use uuid::Uuid;
use valence_protocol::packets::s2c::play::{
    AcknowledgeBlockChange, ClearTitles, CombatDeath, CustomSoundEffect, DisconnectPlay,
    EntityAnimationS2c, EntityEvent, GameEvent, KeepAliveS2c, LoginPlayOwned, OpenScreen,
    PluginMessageS2c, RemoveEntities, ResourcePackS2c, RespawnOwned, SetActionBarText,
    SetCenterChunk, SetContainerContentEncode, SetContainerSlotEncode, SetDefaultSpawnPosition,
    SetEntityMetadata, SetEntityVelocity, SetExperience, SetHeadRotation, SetHealth,
    SetRenderDistance, SetSubtitleText, SetTitleAnimationTimes, SetTitleText,
    SynchronizePlayerPosition, SystemChatMessage, TeleportEntity, UnloadChunk, UpdateAttributes,
    UpdateEntityPosition, UpdateEntityPositionAndRotation, UpdateEntityRotation, UpdateTime,
};
use valence_protocol::types::{
    AttributeProperty, DisplayedSkinParts, GameMode, GameStateChangeReason, SoundCategory,
    SyncPlayerPosLookFlags,
};
use valence_protocol::{
    BlockPos, ByteAngle, Encode, Ident, ItemStack, Packet, RawBytes, Text, Username, VarInt,
};
use vek::Vec3;

use crate::chunk_pos::ChunkPos;
use crate::client::event::next_event_fallible;
use crate::config::Config;
use crate::dimension::DimensionId;
use crate::entity::data::Player;
use crate::entity::{
    self, velocity_to_packet_units, Entities, EntityId, EntityKind, StatusOrAnimation,
};
use crate::inventory::{Inventories, InventoryId};
use crate::player_list::{PlayerListId, PlayerLists};
use crate::player_textures::SignedPlayerTextures;
use crate::server::{NewClientData, PlayPacketReceiver, PlayPacketSender, SharedServer};
use crate::slab_versioned::{Key, VersionedSlab};
use crate::util::{chunks_in_view_distance, is_chunk_in_view_distance};
use crate::world::{WorldId, Worlds};

mod event;

/// A container for all [`Client`]s on a [`Server`](crate::server::Server).
///
/// New clients are automatically inserted into this container but
/// are not automatically deleted. It is your responsibility to delete them once
/// they disconnect. This can be checked with [`Client::is_disconnected`].
pub struct Clients<C: Config> {
    slab: VersionedSlab<Client<C>>,
}

impl<C: Config> Clients<C> {
    pub(crate) fn new() -> Self {
        Self {
            slab: VersionedSlab::new(),
        }
    }

    pub(crate) fn insert(&mut self, client: Client<C>) -> (ClientId, &mut Client<C>) {
        let (k, client) = self.slab.insert(client);
        (ClientId(k), client)
    }

    /// Removes a client from the server.
    ///
    /// If the given client ID is valid, the client's `ClientState` is returned
    /// and the client is deleted. Otherwise, `None` is returned and the
    /// function has no effect.
    pub fn remove(&mut self, client: ClientId) -> Option<C::ClientState> {
        self.slab.remove(client.0).map(|c| {
            info!(username = %c.username, uuid = %c.uuid, ip = %c.ip, "removing client");
            c.state
        })
    }

    /// Deletes all clients from the server for which `f` returns `false`.
    ///
    /// All clients are visited in an unspecified order.
    pub fn retain(&mut self, mut f: impl FnMut(ClientId, &mut Client<C>) -> bool) {
        self.slab.retain(|k, v| {
            if !f(ClientId(k), v) {
                info!(username = %v.username, uuid = %v.uuid, ip = %v.ip, "removing client");
                false
            } else {
                true
            }
        })
    }

    /// Returns the number of clients on the server. This includes clients for
    /// which [`Client::is_disconnected`] returns true.
    pub fn len(&self) -> usize {
        self.slab.len()
    }

    /// Returns `true` if there are no clients on the server. This includes
    /// clients for which [`Client::is_disconnected`] returns true.
    pub fn is_empty(&self) -> bool {
        self.slab.len() == 0
    }

    /// Returns a shared reference to the client with the given ID. If
    /// the ID is invalid, then `None` is returned.
    pub fn get(&self, client: ClientId) -> Option<&Client<C>> {
        self.slab.get(client.0)
    }

    /// Returns an exclusive reference to the client with the given ID. If the
    /// ID is invalid, then `None` is returned.
    pub fn get_mut(&mut self, client: ClientId) -> Option<&mut Client<C>> {
        self.slab.get_mut(client.0)
    }

    /// Returns an iterator over all clients on the server in an unspecified
    /// order.
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (ClientId, &Client<C>)> + FusedIterator + Clone + '_ {
        self.slab.iter().map(|(k, v)| (ClientId(k), v))
    }

    /// Returns a mutable iterator over all clients on the server in an
    /// unspecified order.
    pub fn iter_mut(
        &mut self,
    ) -> impl ExactSizeIterator<Item = (ClientId, &mut Client<C>)> + FusedIterator + '_ {
        self.slab.iter_mut().map(|(k, v)| (ClientId(k), v))
    }

    /// Returns a parallel iterator over all clients on the server in an
    /// unspecified order.
    pub fn par_iter(&self) -> impl ParallelIterator<Item = (ClientId, &Client<C>)> + Clone + '_ {
        self.slab.par_iter().map(|(k, v)| (ClientId(k), v))
    }

    /// Returns a parallel mutable iterator over all clients on the server in an
    /// unspecified order.
    pub fn par_iter_mut(
        &mut self,
    ) -> impl ParallelIterator<Item = (ClientId, &mut Client<C>)> + '_ {
        self.slab.par_iter_mut().map(|(k, v)| (ClientId(k), v))
    }
}

/// An identifier for a [`Client`] on the server.
///
/// Client IDs are either _valid_ or _invalid_. Valid client IDs point to
/// clients that have not been deleted, while invalid IDs point to those that
/// have. Once an ID becomes invalid, it will never become valid again.
///
/// The [`Ord`] instance on this type is correct but otherwise unspecified. This
/// is useful for storing IDs in containers such as
/// [`BTreeMap`](std::collections::BTreeMap).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct ClientId(Key);

impl ClientId {
    /// The value of the default client ID which is always invalid.
    pub const NULL: Self = Self(Key::NULL);
}

/// Represents a remote connection to a client after successfully logging in.
///
/// Much like an [`Entity`], clients possess a location, rotation, and UUID.
/// However, clients are handled separately from entities and are partially
/// managed by the library.
///
/// By default, clients have no influence over the worlds they reside in. They
/// cannot break blocks, hurt entities, or see other clients. Interactions with
/// the server must be handled explicitly with [`Self::next_event`].
///
/// Additionally, clients possess [`Player`] entity data which is only visible
/// to themselves. This can be accessed with [`Self::player`] and
/// [`Self::player_mut`].
///
/// # The Difference Between a "Client" and a "Player"
///
/// Normally in Minecraft, players and clients are one and the same. Players are
/// simply a subtype of the entity base class backed by a remote connection.
///
/// In Valence however, clients and players are decoupled. This separation
/// allows for greater flexibility and enables parallelism.
///
/// [`Entity`]: crate::entity::Entity
pub struct Client<C: Config> {
    /// Custom state.
    pub state: C::ClientState,
    send: Option<PlayPacketSender>,
    recv: PlayPacketReceiver,
    /// Ensures that we don't allow more connections to the server until the
    /// client is dropped.
    _permit: OwnedSemaphorePermit,
    username: Username<String>,
    uuid: Uuid,
    ip: IpAddr,
    textures: Option<SignedPlayerTextures>,
    /// World client is currently in. Default value is **invalid** and must
    /// be set by calling [`Client::respawn`].
    world: WorldId,
    player_list: Option<PlayerListId>,
    /// Player list from the previous tick.
    old_player_list: Option<PlayerListId>,
    position: Vec3<f64>,
    /// Position from the previous tick.
    old_position: Vec3<f64>,
    /// Measured in degrees
    yaw: f32,
    /// Measured in degrees
    pitch: f32,
    view_distance: u8,
    /// Counts up as teleports are made.
    teleport_id_counter: u32,
    /// The number of pending client teleports that have yet to receive a
    /// confirmation. Inbound client position packets should be ignored while
    /// this is nonzero.
    pending_teleports: u32,
    death_location: Option<(DimensionId, BlockPos)>,
    /// The ID of the last keepalive sent.
    last_keepalive_id: u64,
    /// Entities that were visible to this client at the end of the last tick.
    /// This is used to determine what entity create/destroy packets should be
    /// sent.
    loaded_entities: HashSet<EntityId>,
    loaded_chunks: HashSet<ChunkPos>,
    game_mode: GameMode,
    block_change_sequence: i32,
    /// The data for the client's own player entity.
    player_data: Player,
    /// The client's inventory slots.
    slots: Box<[Option<ItemStack>; 45]>,
    /// Contains a set bit for each modified slot in `slots` made by the server
    /// this tick.
    modified_slots: u64,
    /// Counts up as inventory modifications are made by the server. Used to
    /// prevent desync.
    inv_state_id: Wrapping<i32>,
    /// The item currently held by the client's cursor in the inventory.
    cursor_item: Option<ItemStack>,
    /// The currently open inventory. The client can close the screen, making
    /// this [`InventoryId::NULL`].
    open_inventory: InventoryId,
    /// The current window ID. Incremented when inventories are opened.
    window_id: u8,
    bits: ClientBits,
}

#[bitfield(u8)]
struct ClientBits {
    created_this_tick: bool,
    respawn: bool,
    /// If the last sent keepalive got a response.
    got_keepalive: bool,
    hardcore: bool,
    flat: bool,
    respawn_screen: bool,
    cursor_item_modified: bool,
    open_inventory_modified: bool,
    //#[bits(1)]
    //_pad: u8,
}

impl<C: Config> Deref for Client<C> {
    type Target = C::ClientState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<C: Config> DerefMut for Client<C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}

impl<C: Config> Client<C> {
    pub(crate) fn new(
        send: PlayPacketSender,
        recv: PlayPacketReceiver,
        permit: OwnedSemaphorePermit,
        ncd: NewClientData,
        state: C::ClientState,
    ) -> Self {
        Self {
            state,
            send: Some(send),
            recv,
            _permit: permit,
            username: ncd.username,
            uuid: ncd.uuid,
            ip: ncd.ip,
            textures: ncd.textures,
            world: WorldId::default(),
            player_list: None,
            old_player_list: None,
            position: Vec3::default(),
            old_position: Vec3::default(),
            yaw: 0.0,
            pitch: 0.0,
            view_distance: 2,
            teleport_id_counter: 0,
            pending_teleports: 0,
            death_location: None,
            last_keepalive_id: 0,
            loaded_entities: HashSet::new(),
            loaded_chunks: HashSet::new(),
            game_mode: GameMode::Survival,
            block_change_sequence: 0,
            player_data: Player::new(),
            slots: Box::new(array::from_fn(|_| None)),
            modified_slots: 0,
            inv_state_id: Wrapping(0),
            cursor_item: None,
            open_inventory: InventoryId::NULL,
            window_id: 0,
            bits: ClientBits::new()
                .with_got_keepalive(true)
                .with_created_this_tick(true),
        }
    }

    /// Attempts to enqueue a play packet to be sent to this client.
    ///
    /// If encoding the packet fails, the client is disconnected. Has no
    /// effect if the client is already disconnected.
    pub fn queue_packet<P>(&mut self, pkt: &P)
    where
        P: Encode + Packet + ?Sized,
    {
        if let Some(send) = &mut self.send {
            if let Err(e) = send.append_packet(pkt) {
                warn!(
                    username = %self.username,
                    uuid = %self.uuid,
                    ip = %self.ip,
                    "failed to queue packet: {e:#}"
                );
                self.send = None;
            }
        }
    }

    /// If the client joined the game this tick.
    pub fn created_this_tick(&self) -> bool {
        self.bits.created_this_tick()
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

    /// Gets the player textures of this client. If the client does not have
    /// a skin, then `None` is returned.
    pub fn textures(&self) -> Option<&SignedPlayerTextures> {
        self.textures.as_ref()
    }

    /// Gets the world this client is located in.
    pub fn world(&self) -> WorldId {
        self.world
    }

    /// Gets the player list this client sees.
    pub fn player_list(&self) -> Option<&PlayerListId> {
        self.player_list.as_ref()
    }

    /// Sets the player list this client sees.
    ///
    /// The previous player list ID is returned.
    pub fn set_player_list(&mut self, id: impl Into<Option<PlayerListId>>) -> Option<PlayerListId> {
        mem::replace(&mut self.player_list, id.into())
    }

    /// Sets if this client sees the world as superflat. Superflat worlds have
    /// a horizon line lower than normal worlds.
    ///
    /// The player must be (re)spawned for changes to take effect.
    pub fn set_flat(&mut self, flat: bool) {
        self.bits.set_flat(flat);
    }

    /// Gets if this client sees the world as superflat. Superflat worlds have
    /// a horizon line lower than normal worlds.
    pub fn is_flat(&self) -> bool {
        self.bits.flat()
    }

    /// Changes the world this client is located in and respawns the client.
    /// This can be used to respawn the client after death.
    ///
    /// The given [`WorldId`] must be valid. Otherwise, the client is
    /// disconnected.
    pub fn respawn(&mut self, world: WorldId) {
        if self.world != world {
            self.world = world;
            self.bits.set_respawn(true);
        }
    }

    /// Sends a system message to the player which is visible in the chat. The
    /// message is only visible to this client.
    pub fn send_message(&mut self, msg: impl Into<Text>) {
        self.queue_packet(&SystemChatMessage {
            chat: msg.into(),
            kind: VarInt(0),
        });
    }

    pub fn send_plugin_message(&mut self, channel: Ident<&str>, data: &[u8]) {
        self.queue_packet(&PluginMessageS2c {
            channel,
            data: RawBytes(data),
        });
    }

    /// Gets the absolute position of this client in the world it is located
    /// in.
    pub fn position(&self) -> Vec3<f64> {
        self.position
    }

    /// Changes the position and rotation of this client in the world it is
    /// located in.
    ///
    /// If you want to change the client's world, use [`Self::respawn`].
    pub fn teleport(&mut self, pos: impl Into<Vec3<f64>>, yaw: f32, pitch: f32) {
        self.position = pos.into();
        self.yaw = yaw;
        self.pitch = pitch;

        self.queue_packet(&SynchronizePlayerPosition {
            position: self.position.into_array(),
            yaw,
            pitch,
            flags: SyncPlayerPosLookFlags::new(),
            teleport_id: VarInt(self.teleport_id_counter as i32),
            dismount_vehicle: false,
        });

        self.pending_teleports = self.pending_teleports.wrapping_add(1);
        self.teleport_id_counter = self.teleport_id_counter.wrapping_add(1);
    }

    /// Sets the client's velocity in m/s.
    pub fn set_velocity(&mut self, velocity: impl Into<Vec3<f32>>) {
        self.queue_packet(&SetEntityVelocity {
            entity_id: VarInt(0),
            velocity: velocity_to_packet_units(velocity.into()).into_array(),
        })
    }

    /// Gets this client's yaw.
    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    /// Gets this client's pitch.
    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// Sets the spawn position. The client will see `minecraft:compass` items
    /// point at the provided position.
    pub fn set_spawn_position(&mut self, pos: impl Into<BlockPos>, yaw_degrees: f32) {
        self.queue_packet(&SetDefaultSpawnPosition {
            position: pos.into(),
            angle: yaw_degrees,
        });
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

    /// Gets the client's game mode.
    pub fn game_mode(&self) -> GameMode {
        self.game_mode
    }

    /// Sets the client's game mode.
    pub fn set_game_mode(&mut self, game_mode: GameMode) {
        if self.game_mode != game_mode {
            self.game_mode = game_mode;

            if !self.created_this_tick() {
                self.queue_packet(&GameEvent {
                    reason: GameStateChangeReason::ChangeGameMode,
                    value: game_mode as i32 as f32,
                });
            }
        }
    }

    /// Sets whether or not the client sees rain.
    pub fn set_raining(&mut self, raining: bool) {
        self.queue_packet(&GameEvent {
            reason: if raining {
                GameStateChangeReason::BeginRaining
            } else {
                GameStateChangeReason::EndRaining
            },
            value: 0.0,
        });
    }

    /// Sets the client's rain level. This changes the sky color and lightning
    /// on the client.
    ///
    /// The rain level is clamped between `0.0.` and `1.0`.
    pub fn set_rain_level(&mut self, rain_level: f32) {
        self.queue_packet(&GameEvent {
            reason: GameStateChangeReason::RainLevelChange,
            value: rain_level.clamp(0.0, 1.0),
        });
    }

    /// Sets the client's thunder level. This changes the sky color and
    /// lightning on the client.
    ///
    /// For this to take effect, it must already be raining via
    /// [`set_raining`](Self::set_raining) or
    /// [`set_rain_level`](Self::set_rain_level).
    ///
    /// The thunder level is clamped between `0.0` and `1.0`.
    pub fn set_thunder_level(&mut self, thunder_level: f32) {
        self.queue_packet(&GameEvent {
            reason: GameStateChangeReason::ThunderLevelChange,
            value: thunder_level.clamp(0.0, 1.0),
        });
    }

    /// Plays a sound to the client at a given position.
    pub fn play_sound(
        &mut self,
        name: Ident<&str>,
        category: SoundCategory,
        pos: Vec3<f64>,
        volume: f32,
        pitch: f32,
    ) {
        self.queue_packet(&CustomSoundEffect {
            name,
            category,
            position: (pos.as_() * 8).into_array(),
            volume,
            pitch,
            seed: rand::random(),
        });
    }

    /// Sets the title this client sees.
    ///
    /// A title is a large piece of text displayed in the center of the screen
    /// which may also include a subtitle underneath it. The title can be
    /// configured to fade in and out using the [`SetTitleAnimationTimes`]
    /// struct.
    pub fn set_title(
        &mut self,
        title: impl Into<Text>,
        subtitle: impl Into<Text>,
        animation: impl Into<Option<SetTitleAnimationTimes>>,
    ) {
        let title = title.into();
        let subtitle = subtitle.into();

        self.queue_packet(&SetTitleText(title));

        if !subtitle.is_empty() {
            self.queue_packet(&SetSubtitleText(subtitle));
        }

        if let Some(anim) = animation.into() {
            self.queue_packet(&anim);
        }
    }

    /// Sets the action bar for this client.
    pub fn set_action_bar(&mut self, text: impl Into<Text>) {
        self.queue_packet(&SetActionBarText(text.into()));
    }

    /// Sets the attack cooldown speed.
    pub fn set_attack_speed(&mut self, speed: f64) {
        self.queue_packet(&UpdateAttributes {
            entity_id: VarInt(0),
            properties: vec![AttributeProperty {
                key: Ident::new("generic.attack_speed").unwrap(),
                value: speed,
                modifiers: Vec::new(),
            }],
        });
    }

    /// Sets the speed at which the client can run on the ground.
    pub fn set_movement_speed(&mut self, speed: f64) {
        self.queue_packet(&UpdateAttributes {
            entity_id: VarInt(0),
            properties: vec![AttributeProperty {
                key: Ident::new("generic.movement_speed").unwrap(),
                value: speed,
                modifiers: Vec::new(),
            }],
        });
    }

    /// Removes the current title from the client's screen.
    pub fn clear_title(&mut self) {
        self.queue_packet(&ClearTitles { reset: true });
    }

    /// Sets the XP bar visible above hotbar and total experience.
    ///
    /// # Arguments
    /// * `bar` - Floating value in the range `0.0..=1.0` indicating progress on
    ///   the XP bar.
    /// * `level` - Number above the XP bar.
    /// * `total_xp` - TODO.
    pub fn set_level(&mut self, bar: f32, level: i32, total_xp: i32) {
        self.queue_packet(&SetExperience {
            bar,
            level: level.into(),
            total_xp: total_xp.into(),
        })
    }

    /// Sets the health and food of the player.
    /// You can read more about hunger and saturation [here](https://minecraft.fandom.com/wiki/Food#Hunger_vs._Saturation).
    ///
    /// # Arguments
    /// * `health` - Float in range `0.0..=20.0`. Value `<=0` is legal and will
    ///   kill the player.
    /// * `food` - Integer in range `0..=20`.
    /// * `food_saturation` - Float in range `0.0..=5.0`.
    pub fn set_health_and_food(&mut self, health: f32, food: i32, food_saturation: f32) {
        self.queue_packet(&SetHealth {
            health,
            food: food.into(),
            food_saturation,
        })
    }

    /// Kills the client and shows `message` on the death screen. If an entity
    /// killed the player, pass its ID into the function.
    pub fn kill(&mut self, killer: Option<EntityId>, message: impl Into<Text>) {
        self.queue_packet(&CombatDeath {
            player_id: VarInt(0),
            entity_id: killer.map_or(-1, |k| k.to_raw_id()),
            message: message.into(),
        });
    }

    /// Respawns client. Optionally can roll the credits before respawning.
    pub fn win_game(&mut self, show_credits: bool) {
        self.queue_packet(&GameEvent {
            reason: GameStateChangeReason::WinGame,
            value: if show_credits { 1.0 } else { 0.0 },
        });
    }

    pub fn has_respawn_screen(&self) -> bool {
        self.bits.respawn_screen()
    }

    /// Sets whether respawn screen should be displayed after client's death.
    pub fn set_respawn_screen(&mut self, enable: bool) {
        if self.bits.respawn_screen() != enable {
            self.bits.set_respawn_screen(enable);

            if !self.created_this_tick() {
                self.queue_packet(&GameEvent {
                    reason: GameStateChangeReason::EnableRespawnScreen,
                    value: if enable { 0.0 } else { 1.0 },
                });
            }
        }
    }

    pub fn skin_parts(&self) -> DisplayedSkinParts {
        DisplayedSkinParts::new()
            .with_cape(self.player_data.get_cape())
            .with_jacket(self.player_data.get_jacket())
            .with_left_sleeve(self.player_data.get_left_sleeve())
            .with_right_sleeve(self.player_data.get_right_sleeve())
            .with_left_pants_leg(self.player_data.get_left_pants_leg())
            .with_right_pants_leg(self.player_data.get_right_pants_leg())
            .with_hat(self.player_data.get_hat())
    }

    pub fn set_skin_parts(&mut self, parts: DisplayedSkinParts) {
        self.player_data.set_cape(parts.cape());
        self.player_data.set_jacket(parts.jacket());
        self.player_data.set_left_sleeve(parts.left_sleeve());
        self.player_data.set_right_sleeve(parts.right_sleeve());
        self.player_data.set_left_pants_leg(parts.left_pants_leg());
        self.player_data
            .set_right_pants_leg(parts.right_pants_leg());
        self.player_data.set_hat(parts.hat());
    }

    /// Gets whether or not the client is connected to the server.
    ///
    /// A disconnected client object will never become reconnected. It is your
    /// responsibility to remove disconnected clients from the [`Clients`]
    /// container.
    pub fn is_disconnected(&self) -> bool {
        self.send.is_none()
    }

    /// Sends an entity event for the client's own player data.
    pub fn send_entity_event(&mut self, event: entity::EntityEvent) {
        match event.status_or_animation() {
            StatusOrAnimation::Status(code) => self.queue_packet(&EntityEvent {
                entity_id: 0,
                entity_status: code,
            }),
            StatusOrAnimation::Animation(code) => self.queue_packet(&EntityAnimationS2c {
                entity_id: VarInt(0),
                animation: code,
            }),
        }
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
        let dist = dist.clamp(2, 32);

        if self.view_distance != dist {
            self.view_distance = dist;

            if !self.created_this_tick() {
                // Change the render distance fog.
                self.queue_packet(&SetRenderDistance(VarInt(dist as i32)));
            }
        }
    }

    /// Enables hardcore mode. This changes the design of the client's hearts.
    ///
    /// To have any visible effect, this function must be called on the same
    /// tick the client joins the server.
    pub fn set_hardcore(&mut self, hardcore: bool) {
        self.bits.set_hardcore(hardcore);
    }

    /// Gets if hardcore mode is enabled.
    pub fn is_hardcore(&self) -> bool {
        self.bits.hardcore()
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
        self.queue_packet(&ResourcePackS2c {
            url,
            hash,
            forced,
            prompt_message,
        });
    }

    /// Sets the world_age and the current in-game time.
    ///
    /// To stop time from passing, the `time_of_day` parameter must be
    /// negative. The client stops the time at the absolute value.
    pub fn set_time(&mut self, world_age: i64, time_of_day: i64) {
        self.queue_packet(&UpdateTime {
            world_age,
            time_of_day,
        });
    }

    /// Disconnects this client from the server with the provided reason. This
    /// has no effect if the client is already disconnected.
    ///
    /// All future calls to [`Self::is_disconnected`] will return `true`.
    pub fn disconnect(&mut self, reason: impl Into<Text>) {
        self.queue_packet(&DisconnectPlay {
            reason: reason.into(),
        });
        self.disconnect_abrupt();
    }

    /// Like [`Self::disconnect`], but no reason for the disconnect is
    /// sent to the client.
    pub fn disconnect_abrupt(&mut self) {
        self.send = None;
    }

    /// Returns an immutable reference to the client's own [`Player`] data.
    pub fn player(&self) -> &Player {
        &self.player_data
    }

    /// Returns a mutable reference to the client's own [`Player`] data.
    ///
    /// Changes made to this data is only visible to this client.
    pub fn player_mut(&mut self) -> &mut Player {
        &mut self.player_data
    }

    pub fn slot(&self, idx: u16) -> Option<&ItemStack> {
        self.slots
            .get(idx as usize)
            .expect("slot index out of range")
            .as_ref()
    }

    pub fn replace_slot(
        &mut self,
        idx: u16,
        item: impl Into<Option<ItemStack>>,
    ) -> Option<ItemStack> {
        assert!((idx as usize) < self.slots.len(), "slot index out of range");

        let new = item.into();
        let old = &mut self.slots[idx as usize];

        if new != *old {
            self.modified_slots |= 1 << idx;
        }

        mem::replace(old, new)
    }

    pub fn cursor_item(&self) -> Option<&ItemStack> {
        self.cursor_item.as_ref()
    }

    pub fn replace_cursor_item(&mut self, item: impl Into<Option<ItemStack>>) -> Option<ItemStack> {
        let new = item.into();
        if self.cursor_item != new {
            todo!("set cursor item bit");
        }

        mem::replace(&mut self.cursor_item, new)
    }

    pub fn open_inventory(&self) -> InventoryId {
        self.open_inventory
    }

    pub fn set_open_inventory(&mut self, id: InventoryId) {
        if self.open_inventory != id {
            self.bits.set_open_inventory_modified(true);
            self.open_inventory = id;
        }
    }

    pub fn next_event(&mut self) -> Option<ClientEvent> {
        match next_event_fallible(self) {
            Ok(event) => event,
            Err(e) => {
                warn!(
                    username = %self.username,
                    uuid = %self.uuid,
                    ip = %self.ip,
                    "failed to get next event: {e:#}"
                );
                self.send = None;
                None
            }
        }
    }

    pub(crate) fn prepare_c2s_packets(&mut self) {
        if !self.recv.try_recv() {
            self.disconnect_abrupt();
        }
    }

    pub(crate) fn update(
        &mut self,
        shared: &SharedServer<C>,
        entities: &Entities<C>,
        worlds: &Worlds<C>,
        player_lists: &PlayerLists<C>,
        inventories: &Inventories<C>,
    ) {
        if let Some(mut send) = self.send.take() {
            match self.update_fallible(
                &mut send,
                shared,
                entities,
                worlds,
                player_lists,
                inventories,
            ) {
                Ok(()) => self.send = Some(send),
                Err(e) => {
                    let _ = send.append_packet(&DisconnectPlay { reason: "".into() });
                    warn!(
                        username = %self.username,
                        uuid = %self.uuid,
                        ip = %self.ip,
                        "error updating client: {e:#}"
                    );
                }
            }
        }

        self.bits.set_created_this_tick(false);
    }

    /// Called by [`Self::update`] with the possibility of exiting early with an
    /// error. If an error does occur, the client is abruptly disconnected and
    /// the error is reported.
    fn update_fallible(
        &mut self,
        send: &mut PlayPacketSender,
        shared: &SharedServer<C>,
        entities: &Entities<C>,
        worlds: &Worlds<C>,
        player_lists: &PlayerLists<C>,
        inventories: &Inventories<C>,
    ) -> anyhow::Result<()> {
        let world = match worlds.get(self.world) {
            Some(world) => world,
            None => bail!("client is in an invalid world and must be disconnected"),
        };

        let current_tick = shared.current_tick();

        // Send the login (play) packet and other initial packets. We defer this until
        // now so that the user can set the client's initial location, game
        // mode, etc.
        if self.created_this_tick() {
            self.bits.set_respawn(false);

            let dimension_names: Vec<_> = shared
                .dimensions()
                .map(|(id, _)| id.dimension_name())
                .collect();

            // The login packet is prepended so that it is sent before all the other
            // packets. Some packets don't work correctly when sent before the login packet,
            // which is why we're doing this.
            send.prepend_packet(&LoginPlayOwned {
                entity_id: 0, // ID 0 is reserved for clients.
                is_hardcore: self.bits.hardcore(),
                game_mode: self.game_mode,
                previous_game_mode: -1,
                dimension_names,
                registry_codec: shared.registry_codec().clone(),
                dimension_type_name: world.meta.dimension().dimension_type_name(),
                dimension_name: world.meta.dimension().dimension_name(),
                hashed_seed: 10,
                max_players: VarInt(0), // Unused
                view_distance: VarInt(self.view_distance() as i32),
                simulation_distance: VarInt(16),
                reduced_debug_info: false,
                enable_respawn_screen: self.bits.respawn_screen(),
                is_debug: false,
                is_flat: self.bits.flat(),
                last_death_location: self
                    .death_location
                    .map(|(id, pos)| (id.dimension_name(), pos)),
            })?;

            if let Some(id) = &self.player_list {
                player_lists.get(id).send_initial_packets(send)?;
            }
        } else {
            if self.bits.respawn() {
                self.bits.set_respawn(false);

                // TODO: changing worlds didn't unload entities?
                //self.loaded_entities.clear();
                self.loaded_chunks.clear();

                /*
                // Client bug workaround: send the client to a dummy dimension first.
                // TODO: is there actually a bug?
                send.append_packet(&RespawnOwned {
                    dimension_type_name: DimensionId(0).dimension_type_name(),
                    dimension_name: ident!("{LIBRARY_NAMESPACE}:dummy_dimension"),
                    hashed_seed: 0,
                    game_mode: self.game_mode(),
                    previous_game_mode: -1,
                    is_debug: false,
                    is_flat: self.bits.flat(),
                    copy_metadata: true,
                    last_death_location: None,
                })?;
                 */

                send.append_packet(&RespawnOwned {
                    dimension_type_name: world.meta.dimension().dimension_type_name(),
                    dimension_name: world.meta.dimension().dimension_name(),
                    hashed_seed: 0,
                    game_mode: self.game_mode(),
                    previous_game_mode: -1,
                    is_debug: false,
                    is_flat: self.bits.flat(),
                    copy_metadata: true,
                    last_death_location: self
                        .death_location
                        .map(|(id, pos)| (id.dimension_name(), pos)),
                })?;
            }

            // If the player list was changed...
            if self.old_player_list != self.player_list {
                // Delete existing entries from old player list.
                if let Some(id) = &self.old_player_list {
                    player_lists.get(id).queue_clear_packets(send)?;
                }

                // Get initial packets for new player list.
                if let Some(id) = &self.player_list {
                    player_lists.get(id).send_initial_packets(send)?;
                }

                self.old_player_list = self.player_list.clone();
            } else if let Some(id) = &self.player_list {
                // Otherwise, update current player list.
                player_lists.get(id).send_update_packets(send)?;
            }
        }

        // Check if it's time to send another keepalive.
        if current_tick % (shared.tick_rate() * 10) == 0 {
            if self.bits.got_keepalive() {
                let id = rand::random();
                send.append_packet(&KeepAliveS2c { id })?;
                self.last_keepalive_id = id;
                self.bits.set_got_keepalive(false);
            } else {
                bail!("timed out (no keepalive response)");
            }
        }

        let center = ChunkPos::at(self.position.x, self.position.z);

        // Send the update view position packet if the client changes the chunk they're
        // in.
        if ChunkPos::at(self.old_position.x, self.old_position.z) != center {
            send.append_packet(&SetCenterChunk {
                chunk_x: VarInt(center.x),
                chunk_z: VarInt(center.z),
            })?;
        }

        let dimension = shared.dimension(world.meta.dimension());

        // Update existing chunks and unload those outside the view distance. Chunks
        // that have been overwritten also need to be unloaded.
        // TODO: don't ignore errors in closure.
        self.loaded_chunks.retain(|&pos| {
            // The cache stops chunk data packets from needing to be sent when a player
            // moves to an adjacent chunk and back to the original.
            let cache = 2;

            if let Some(chunk) = world.chunks.get(pos) {
                if is_chunk_in_view_distance(center, pos, self.view_distance + cache)
                    && !chunk.created_this_tick()
                {
                    let _ = chunk.block_change_packets(pos, dimension.min_y, send);
                    return true;
                }
            }

            let _ = send.append_packet(&UnloadChunk {
                chunk_x: pos.x,
                chunk_z: pos.z,
            });

            false
        });

        // Load new chunks within the view distance
        {
            let mut scratch = Vec::new();
            let biome_registry_len = shared.biomes().len();

            for pos in chunks_in_view_distance(center, self.view_distance) {
                if let Some(chunk) = world.chunks.get(pos) {
                    if self.loaded_chunks.insert(pos) {
                        chunk.chunk_data_packet(send, &mut scratch, pos, biome_registry_len)?;
                    }
                }
            }
        }

        // Acknowledge broken/placed blocks.
        if self.block_change_sequence != 0 {
            send.append_packet(&AcknowledgeBlockChange {
                sequence: VarInt(self.block_change_sequence),
            })?;

            self.block_change_sequence = 0;
        }

        let mut entities_to_unload = Vec::new();

        // Update all entities that are visible and unload entities that are no
        // longer visible.
        // TODO: don't ignore errors in the closure.
        self.loaded_entities.retain(|&id| {
            if let Some(entity) = entities.get(id) {
                debug_assert!(entity.kind() != EntityKind::Marker);
                if self.world == entity.world()
                    && self.position.distance(entity.position()) <= self.view_distance as f64 * 16.0
                {
                    let _ = entity.send_updated_tracked_data(send, id);

                    let position_delta = entity.position() - entity.old_position();
                    let needs_teleport = position_delta.map(f64::abs).reduce_partial_max() >= 8.0;
                    let flags = entity.bits();

                    if entity.position() != entity.old_position()
                        && !needs_teleport
                        && flags.yaw_or_pitch_modified()
                    {
                        let _ = send.append_packet(&UpdateEntityPositionAndRotation {
                            entity_id: VarInt(id.to_raw_id()),
                            delta: (position_delta * 4096.0).as_::<i16>().into_array(),
                            yaw: ByteAngle::from_degrees(entity.yaw()),
                            pitch: ByteAngle::from_degrees(entity.pitch()),
                            on_ground: entity.on_ground(),
                        });
                    } else {
                        if entity.position() != entity.old_position() && !needs_teleport {
                            let _ = send.append_packet(&UpdateEntityPosition {
                                entity_id: VarInt(id.to_raw_id()),
                                delta: (position_delta * 4096.0).as_::<i16>().into_array(),
                                on_ground: entity.on_ground(),
                            });
                        }

                        if flags.yaw_or_pitch_modified() {
                            let _ = send.append_packet(&UpdateEntityRotation {
                                entity_id: VarInt(id.to_raw_id()),
                                yaw: ByteAngle::from_degrees(entity.yaw()),
                                pitch: ByteAngle::from_degrees(entity.pitch()),
                                on_ground: entity.on_ground(),
                            });
                        }
                    }

                    if needs_teleport {
                        let _ = send.append_packet(&TeleportEntity {
                            entity_id: VarInt(id.to_raw_id()),
                            position: entity.position().into_array(),
                            yaw: ByteAngle::from_degrees(entity.yaw()),
                            pitch: ByteAngle::from_degrees(entity.pitch()),
                            on_ground: entity.on_ground(),
                        });
                    }

                    if flags.velocity_modified() {
                        let _ = send.append_packet(&SetEntityVelocity {
                            entity_id: VarInt(id.to_raw_id()),
                            velocity: velocity_to_packet_units(entity.velocity()).into_array(),
                        });
                    }

                    if flags.head_yaw_modified() {
                        let _ = send.append_packet(&SetHeadRotation {
                            entity_id: VarInt(id.to_raw_id()),
                            head_yaw: ByteAngle::from_degrees(entity.head_yaw()),
                        });
                    }

                    let _ = send_entity_events(send, id.to_raw_id(), entity.events());

                    return true;
                }
            }

            entities_to_unload.push(VarInt(id.to_raw_id()));
            false
        });

        if !entities_to_unload.is_empty() {
            send.append_packet(&RemoveEntities {
                entity_ids: entities_to_unload,
            })?;
        }

        // Update the client's own player metadata.
        let mut data = Vec::new();
        self.player_data.updated_tracked_data(&mut data);

        if !data.is_empty() {
            data.push(0xff);

            send.append_packet(&SetEntityMetadata {
                entity_id: VarInt(0),
                metadata: RawBytes(&data),
            })?;
        }

        // Spawn new entities within the view distance.
        let pos = self.position();
        let view_dist = self.view_distance;
        self.player_data.clear_modifications();

        if let Some(e) = world.spatial_index.query(
            |bb| bb.projected_point(pos).distance(pos) <= view_dist as f64 * 16.0,
            |id, _| {
                let entity = entities
                    .get(id)
                    .expect("entity IDs in spatial index should be valid at this point");

                if entity.kind() != EntityKind::Marker
                    && entity.uuid() != self.uuid
                    && self.loaded_entities.insert(id)
                {
                    if let Err(e) = entity.send_spawn_packets(id, send) {
                        return Some(e);
                    }

                    if let Err(e) = entity.send_initial_tracked_data(send, id) {
                        return Some(e);
                    }

                    if let Err(e) = send_entity_events(send, id.to_raw_id(), entity.events()) {
                        return Some(e);
                    }
                }

                None
            },
        ) {
            return Err(e);
        }

        // Update the client's own inventory.
        if self.modified_slots != 0 {
            if self.created_this_tick()
                || self.modified_slots == u64::MAX && self.bits.cursor_item_modified()
            {
                // Update the whole inventory.
                send.append_packet(&SetContainerContentEncode {
                    window_id: 0,
                    state_id: VarInt(self.inv_state_id.0),
                    slots: self.slots.as_slice(),
                    carried_item: &self.cursor_item,
                })?;

                self.inv_state_id += 1;
                self.bits.set_cursor_item_modified(false);
            } else {
                // Update only the slots that were modified.
                for (i, slot) in self.slots.iter().enumerate() {
                    if (self.modified_slots >> i) & 1 == 1 {
                        send.append_packet(&SetContainerSlotEncode {
                            window_id: 0,
                            state_id: VarInt(self.inv_state_id.0),
                            slot_idx: i as i16,
                            slot_data: slot.as_ref(),
                        })?;

                        self.inv_state_id += 1;
                    }
                }
            }

            self.modified_slots = 0;
        }

        if self.bits.cursor_item_modified() {
            self.bits.set_cursor_item_modified(false);

            send.append_packet(&SetContainerSlotEncode {
                window_id: -1,
                state_id: VarInt(self.inv_state_id.0),
                slot_idx: -1,
                slot_data: self.cursor_item.as_ref(),
            })?;

            self.inv_state_id += 1;
        }

        // Update the window the client has opened.
        if self.bits.open_inventory_modified() {
            // Open a new window.
            self.bits.set_open_inventory_modified(false);

            if let Some(inv) = inventories.get(self.open_inventory) {
                self.window_id = self.window_id % 100 + 1;
                self.inv_state_id += 1;

                send.append_packet(&OpenScreen {
                    window_id: VarInt(self.window_id.into()),
                    window_type: VarInt(inv.kind() as i32),
                    window_title: inv.title().clone(),
                })?;

                send.append_packet(&SetContainerContentEncode {
                    window_id: self.window_id,
                    state_id: VarInt(self.inv_state_id.0),
                    slots: inv.slot_slice(),
                    carried_item: &self.cursor_item,
                })?;
            }
        } else {
            // Update an already open window.
            if let Some(inv) = inventories.get(self.open_inventory) {
                inv.send_update(send, self.window_id, &mut self.inv_state_id)?;
            }
        }

        // TODO: send close screen packet under what circumstances?

        self.old_position = self.position;

        send.flush().context("failed to flush packet queue")?;

        Ok(())
    }
}

fn send_entity_events(
    send: &mut PlayPacketSender,
    entity_id: i32,
    events: &[entity::EntityEvent],
) -> anyhow::Result<()> {
    for &event in events {
        match event.status_or_animation() {
            StatusOrAnimation::Status(code) => send.append_packet(&EntityEvent {
                entity_id,
                entity_status: code,
            })?,
            StatusOrAnimation::Animation(code) => send.append_packet(&EntityAnimationS2c {
                entity_id: VarInt(entity_id),
                animation: code,
            })?,
        }
    }

    Ok(())
}
