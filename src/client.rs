//! Connections to the server after logging in.

use std::collections::{HashSet, VecDeque};
use std::iter::FusedIterator;
use std::mem;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub use bitfield_struct::bitfield;
pub use event::*;
use flume::{Receiver, Sender, TrySendError};
use rayon::iter::ParallelIterator;
use uuid::Uuid;
use vek::Vec3;

use crate::block_pos::BlockPos;
use crate::chunk_pos::ChunkPos;
use crate::config::Config;
use crate::dimension::DimensionId;
use crate::entity::data::Player;
use crate::entity::{
    self, velocity_to_packet_units, Entities, EntityId, EntityKind, StatusOrAnimation,
};
use crate::ident::Ident;
use crate::inventory::{Inventory, PlayerInventory};
use crate::player_list::{PlayerListId, PlayerLists};
use crate::player_textures::SignedPlayerTextures;
use crate::protocol::packets::c2s::play::{self, C2sPlayPacket, InteractKind, PlayerCommandId};
pub use crate::protocol::packets::s2c::play::SetTitleAnimationTimes;
use crate::protocol::packets::s2c::play::{
    AcknowledgeBlockChange, ClearTitles, CustomSoundEffect, DisconnectPlay, EntityAnimationS2c,
    EntityAttributesProperty, EntityEvent, GameEvent, GameStateChangeReason, KeepAliveS2c,
    LoginPlay, PlayerPositionLookFlags, RemoveEntities, ResourcePackS2c, Respawn, S2cPlayPacket,
    SetActionBarText, SetCenterChunk, SetDefaultSpawnPosition, SetEntityMetadata,
    SetEntityVelocity, SetExperience, SetHeadRotation, SetHealth, SetRenderDistance,
    SetSubtitleText, SetTitleText, SoundCategory, SynchronizePlayerPosition, SystemChatMessage,
    TeleportEntity, UnloadChunk, UpdateAttributes, UpdateEntityPosition,
    UpdateEntityPositionAndRotation, UpdateEntityRotation, UpdateTime,
};
use crate::protocol::{BoundedInt, BoundedString, ByteAngle, RawBytes, Slot, SlotId, VarInt};
use crate::server::{C2sPacketChannels, NewClientData, S2cPlayMessage, SharedServer};
use crate::slab_versioned::{Key, VersionedSlab};
use crate::text::Text;
use crate::util::{chunks_in_view_distance, is_chunk_in_view_distance};
use crate::world::{WorldId, Worlds};
use crate::{ident, LIBRARY_NAMESPACE};

/// Contains the [`ClientEvent`] enum and related data types.
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
        self.slab.remove(client.0).map(|c| c.state)
    }

    /// Deletes all clients from the server for which `f` returns `false`.
    ///
    /// All clients are visited in an unspecified order.
    pub fn retain(&mut self, mut f: impl FnMut(ClientId, &mut Client<C>) -> bool) {
        self.slab.retain(|k, v| f(ClientId(k), v))
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
/// the server must be handled explicitly with [`Self::pop_event`].
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
    /// Setting this to `None` disconnects the client.
    send: SendOpt,
    recv: Receiver<C2sPlayPacket>,
    uuid: Uuid,
    username: String,
    textures: Option<SignedPlayerTextures>,
    world: WorldId,
    player_list: Option<PlayerListId>,
    old_player_list: Option<PlayerListId>,
    position: Vec3<f64>,
    old_position: Vec3<f64>,
    /// Measured in m/s.
    velocity: Vec3<f32>,
    /// Measured in degrees
    yaw: f32,
    /// Measured in degrees
    pitch: f32,
    view_distance: u8,
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
    /// Entities that were visible to this client at the end of the last tick.
    /// This is used to determine what entity create/destroy packets should be
    /// sent.
    loaded_entities: HashSet<EntityId>,
    loaded_chunks: HashSet<ChunkPos>,
    new_game_mode: GameMode,
    old_game_mode: GameMode,
    settings: Option<Settings>,
    dug_block_sequence: i32,
    /// Should be sent after login packet.
    msgs_to_send: Vec<Text>,
    bar_to_send: Option<Text>,
    resource_pack_to_send: Option<ResourcePackS2c>,
    attack_speed: f64,
    movement_speed: f64,
    pub inventory: Arc<Mutex<PlayerInventory>>,
    bits: ClientBits,
    /// The data for the client's own player entity.
    player_data: Player,
    entity_events: Vec<entity::EntityEvent>,
    /// The item currently being held by the client's cursor in an inventory
    /// screen. Does not work for creative mode.
    cursor_held_item: Slot,
    selected_hotbar_slot: SlotId,
}

#[bitfield(u16)]
struct ClientBits {
    spawn: bool,
    flat: bool,
    teleported_this_tick: bool,
    /// If spawn_position or spawn_position_yaw were modified this tick.
    modified_spawn_position: bool,
    /// If the last sent keepalive got a response.
    got_keepalive: bool,
    hardcore: bool,
    attack_speed_modified: bool,
    movement_speed_modified: bool,
    velocity_modified: bool,
    created_this_tick: bool,
    view_distance_modified: bool,
    #[bits(5)]
    _pad: u8,
}

impl<C: Config> Client<C> {
    pub(crate) fn new(
        packet_channels: C2sPacketChannels,
        ncd: NewClientData,
        state: C::ClientState,
    ) -> Self {
        let (send, recv) = packet_channels;

        Self {
            state,
            send: Some(send),
            recv,
            uuid: ncd.uuid,
            username: ncd.username,
            textures: ncd.textures,
            world: WorldId::default(),
            old_player_list: None,
            player_list: None,
            position: Vec3::default(),
            old_position: Vec3::default(),
            velocity: Vec3::default(),
            yaw: 0.0,
            pitch: 0.0,
            view_distance: 2,
            teleport_id_counter: 0,
            pending_teleports: 0,
            spawn_position: BlockPos::default(),
            spawn_position_yaw: 0.0,
            death_location: None,
            events: VecDeque::new(),
            last_keepalive_id: 0,
            loaded_entities: HashSet::new(),
            loaded_chunks: HashSet::new(),
            new_game_mode: GameMode::Survival,
            old_game_mode: GameMode::Survival,
            settings: None,
            dug_block_sequence: 0,
            msgs_to_send: Vec::new(),
            bar_to_send: None,
            resource_pack_to_send: None,
            attack_speed: 4.0,
            movement_speed: 0.7,
            inventory: Arc::new(Mutex::new(PlayerInventory::default())),
            bits: ClientBits::new()
                .with_modified_spawn_position(true)
                .with_got_keepalive(true)
                .with_created_this_tick(true),
            player_data: Player::new(),
            entity_events: Vec::new(),
            cursor_held_item: None,
            selected_hotbar_slot: 36,
        }
    }

    /// If the client joined the game this tick.
    pub fn created_this_tick(&self) -> bool {
        self.bits.created_this_tick()
    }

    /// Gets the client's UUID.
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    /// Gets the username of this client.
    pub fn username(&self) -> &str {
        &self.username
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
    pub fn spawn(&mut self, world: WorldId) {
        self.world = world;
        self.bits.set_spawn(true);
    }

    /// Sends a system message to the player which is visible in the chat. The
    /// message is only visible to this client.
    pub fn send_message(&mut self, msg: impl Into<Text>) {
        // We buffer messages because weird things happen if we send them before the
        // login packet.
        self.msgs_to_send.push(msg.into());
    }

    /// Gets the absolute position of this client in the world it is located
    /// in.
    pub fn position(&self) -> Vec3<f64> {
        self.position
    }

    /// Changes the position and rotation of this client in the world it is
    /// located in.
    ///
    /// If you want to change the client's world, use [`Self::spawn`].
    pub fn teleport(&mut self, pos: impl Into<Vec3<f64>>, yaw: f32, pitch: f32) {
        self.position = pos.into();
        self.yaw = yaw;
        self.pitch = pitch;

        self.bits.set_teleported_this_tick(true);
    }

    /// Gets the most recently set velocity of this client in m/s.
    pub fn velocity(&self) -> Vec3<f32> {
        self.velocity
    }

    /// Sets the client's velocity in m/s.
    pub fn set_velocity(&mut self, velocity: impl Into<Vec3<f32>>) {
        self.velocity = velocity.into();
        self.bits.set_velocity_modified(true);
    }

    /// Gets this client's yaw.
    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    /// Gets this client's pitch.
    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// Gets the spawn position. The client will see `minecraft:compass` items
    /// point at the returned position.
    pub fn spawn_position(&self) -> BlockPos {
        self.spawn_position
    }

    /// Sets the spawn position. The client will see `minecraft:compass` items
    /// point at the provided position.
    pub fn set_spawn_position(&mut self, pos: impl Into<BlockPos>, yaw_degrees: f32) {
        let pos = pos.into();
        if pos != self.spawn_position || yaw_degrees != self.spawn_position_yaw {
            self.spawn_position = pos;
            self.spawn_position_yaw = yaw_degrees;
            self.bits.set_modified_spawn_position(true);
        }
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
        self.new_game_mode
    }

    /// Sets the client's game mode.
    pub fn set_game_mode(&mut self, game_mode: GameMode) {
        self.new_game_mode = game_mode;
    }

    /// Sets whether or not the client sees rain.
    pub fn set_raining(&mut self, raining: bool) {
        self.send_packet(GameEvent {
            reason: if raining {
                GameStateChangeReason::BeginRaining
            } else {
                GameStateChangeReason::EndRaining
            },
            value: 0.0,
        })
    }

    /// Sets the client's rain level. This changes the sky color and lightning
    /// on the client.
    ///
    /// The rain level is clamped between `0.0.` and `1.0`.
    pub fn set_rain_level(&mut self, rain_level: f32) {
        self.send_packet(GameEvent {
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
        self.send_packet(GameEvent {
            reason: GameStateChangeReason::ThunderLevelChange,
            value: thunder_level.clamp(0.0, 1.0),
        });
    }

    /// Plays a sound to the client at a given position.
    pub fn play_sound(
        &mut self,
        name: Ident<'static>,
        category: SoundCategory,
        pos: Vec3<f64>,
        volume: f32,
        pitch: f32,
    ) {
        self.send_packet(CustomSoundEffect {
            name,
            category,
            position: pos.as_() * 8,
            volume,
            pitch,
            seed: 0,
        });
    }

    /// Sets the title this client sees.
    ///
    /// A title is a large piece of text displayed in the center of the screen
    /// which may also include a subtitle underneath it. The title
    /// can be configured to fade in and out using the
    /// [`SetTitleAnimationTimes`] struct.
    pub fn set_title(
        &mut self,
        title: impl Into<Text>,
        subtitle: impl Into<Text>,
        animation: impl Into<Option<SetTitleAnimationTimes>>,
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

    /// Sets the action bar for this client.
    pub fn set_action_bar(&mut self, text: impl Into<Text>) {
        self.bar_to_send = Some(text.into());
    }

    /// Gets the attack cooldown speed.
    pub fn attack_speed(&self) -> f64 {
        self.attack_speed
    }

    /// Sets the attack cooldown speed.
    pub fn set_attack_speed(&mut self, speed: f64) {
        if self.attack_speed != speed {
            self.attack_speed = speed;
            self.bits.set_attack_speed_modified(true);
        }
    }

    /// Gets the speed at which the client can run on the ground.
    pub fn movement_speed(&self) -> f64 {
        self.movement_speed
    }

    /// Sets the speed at which the client can run on the ground.
    pub fn set_movement_speed(&mut self, speed: f64) {
        if self.movement_speed != speed {
            self.movement_speed = speed;
            self.bits.set_movement_speed_modified(true);
        }
    }

    /// Removes the current title from the client's screen.
    pub fn clear_title(&mut self) {
        self.send_packet(ClearTitles { reset: true });
    }

    /// Sets the XP bar visible above hotbar and total experience.
    ///
    /// # Arguments
    /// * `bar` - Floating value in the range `0.0..=1.0` indicating progress on
    ///   the XP bar.
    /// * `level` - Number above the XP bar.
    /// * `total_xp` - TODO.
    pub fn set_level(&mut self, bar: f32, level: i32, total_xp: i32) {
        self.send_packet(SetExperience {
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
        self.send_packet(SetHealth {
            health,
            food: food.into(),
            food_saturation,
        })
    }

    /// Gets whether or not the client is connected to the server.
    ///
    /// A disconnected client object will never become reconnected. It is your
    /// responsibility to remove disconnected clients from the [`Clients`]
    /// container.
    pub fn is_disconnected(&self) -> bool {
        self.send.is_none()
    }

    /// Returns an iterator over all pending client events in the order they
    /// will be removed from the queue.
    pub fn events(
        &self,
    ) -> impl DoubleEndedIterator<Item = &ClientEvent> + ExactSizeIterator + FusedIterator + Clone + '_
    {
        self.events.iter()
    }

    /// Removes a [`ClientEvent`] from the event queue.
    ///
    /// If there are no remaining events, `None` is returned.
    ///
    /// Any remaining client events are deleted at the end of the
    /// current tick.
    pub fn pop_event(&mut self) -> Option<ClientEvent> {
        self.events.pop_front()
    }

    /// Pushes an entity event to the queue.
    pub fn push_entity_event(&mut self, event: entity::EntityEvent) {
        self.entity_events.push(event);
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
            self.bits.set_view_distance_modified(true);
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
        url: impl Into<String>,
        hash: impl Into<String>,
        forced: bool,
        prompt_message: impl Into<Option<Text>>,
    ) {
        self.resource_pack_to_send = Some(ResourcePackS2c {
            url: url.into(),
            hash: BoundedString(hash.into()),
            forced,
            prompt_message: prompt_message.into(),
        });
    }

    /// Sets the world_age and the current in-game time.
    ///
    /// To stop time from passing, the `time_of_day` parameter must be
    /// negative. The client stops the time at the absolute value.
    pub fn set_time(&mut self, world_age: i64, time_of_day: i64) {
        self.send_packet(UpdateTime {
            world_age,
            time_of_day,
        });
    }

    /// Gets the client's current settings.
    pub fn settings(&self) -> Option<&Settings> {
        self.settings.as_ref()
    }

    pub fn held_item(&self) -> Slot {
        self.inventory
            .lock()
            .unwrap()
            .get_slot(self.selected_hotbar_slot)
    }

    /// Disconnects this client from the server with the provided reason. This
    /// has no effect if the client is already disconnected.
    ///
    /// All future calls to [`Self::is_disconnected`] will return `true`.
    pub fn disconnect(&mut self, reason: impl Into<Text>) {
        if self.send.is_some() {
            let txt = reason.into();
            log::info!("disconnecting client '{}': \"{txt}\"", self.username);

            self.send_packet(DisconnectPlay { reason: txt });

            self.send = None;
        }
    }

    /// Like [`Self::disconnect`], but no reason for the disconnect is
    /// displayed.
    pub fn disconnect_no_reason(&mut self) {
        if self.send.is_some() {
            log::info!("disconnecting client '{}'", self.username);
            self.send = None;
        }
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

    /// Attempts to enqueue a play packet to be sent to this client. The client
    /// is disconnected if the clientbound packet buffer is full.
    pub fn send_packet(&mut self, packet: impl Into<S2cPlayPacket>) {
        send_packet(&mut self.send, packet);
    }

    pub(crate) fn handle_serverbound_packets(&mut self, entities: &Entities<C>) {
        self.events.clear();
        for _ in 0..self.recv.len() {
            self.handle_serverbound_packet(entities, self.recv.try_recv().unwrap());
        }
    }

    fn handle_serverbound_packet(&mut self, entities: &Entities<C>, pkt: C2sPlayPacket) {
        match pkt {
            C2sPlayPacket::ConfirmTeleport(p) => {
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
                }
            }
            C2sPlayPacket::QueryBlockEntityTag(_) => {}
            C2sPlayPacket::ChangeDifficulty(_) => {}
            C2sPlayPacket::MessageAcknowledgment(_) => {}
            C2sPlayPacket::ChatCommand(_) => {}
            C2sPlayPacket::ChatMessage(p) => self.events.push_back(ClientEvent::ChatMessage {
                message: p.message.0,
                timestamp: Duration::from_millis(p.timestamp),
            }),
            C2sPlayPacket::ChatPreviewC2s(_) => {}
            C2sPlayPacket::ClientCommand(_) => {}
            C2sPlayPacket::ClientInformation(p) => {
                self.events.push_back(ClientEvent::SettingsChanged {
                    locale: p.locale.0,
                    view_distance: p.view_distance.0,
                    chat_mode: p.chat_mode,
                    chat_colors: p.chat_colors,
                    main_hand: p.main_hand,
                    displayed_skin_parts: p.displayed_skin_parts,
                    allow_server_listings: p.allow_server_listings,
                })
            }
            C2sPlayPacket::CommandSuggestionsRequest(_) => {}
            C2sPlayPacket::ClickContainerButton(_) => {}
            C2sPlayPacket::ClickContainer(p) => {
                if p.slot_idx == -999 {
                    // client is trying to drop the currently held stack
                    let held = std::mem::replace(&mut self.cursor_held_item, None);
                    match held {
                        None => {}
                        Some(stack) => self.events.push_back(ClientEvent::DropItemStack { stack }),
                    }
                } else {
                    self.cursor_held_item = p.carried_item.clone();
                    self.events.push_back(ClientEvent::ClickContainer {
                        window_id: p.window_id,
                        state_id: p.state_id,
                        slot_id: p.slot_idx,
                        mode: p.mode,
                        slot_changes: p.slots,
                        carried_item: p.carried_item,
                    });
                }
            }
            C2sPlayPacket::CloseContainerC2s(c) => {
                self.events.push_back(ClientEvent::CloseScreen {
                    window_id: c.window_id,
                })
            }
            C2sPlayPacket::PluginMessageC2s(_) => {}
            C2sPlayPacket::EditBook(_) => {}
            C2sPlayPacket::QueryEntityTag(_) => {}
            C2sPlayPacket::Interact(p) => {
                if let Some(id) = entities.get_with_network_id(p.entity_id.0) {
                    self.events.push_back(ClientEvent::InteractWithEntity {
                        id,
                        sneaking: p.sneaking,
                        kind: match p.kind {
                            InteractKind::Interact(hand) => InteractWithEntityKind::Interact(hand),
                            InteractKind::Attack => InteractWithEntityKind::Attack,
                            InteractKind::InteractAt((target, hand)) => {
                                InteractWithEntityKind::InteractAt { target, hand }
                            }
                        },
                    });
                }
            }
            C2sPlayPacket::JigsawGenerate(_) => {}
            C2sPlayPacket::KeepAliveC2s(p) => {
                let last_keepalive_id = self.last_keepalive_id;
                if self.bits.got_keepalive() {
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
                    self.bits.set_got_keepalive(true);
                }
            }
            C2sPlayPacket::LockDifficulty(_) => {}
            C2sPlayPacket::SetPlayerPosition(p) => {
                if self.pending_teleports == 0 {
                    self.position = p.position;

                    self.events.push_back(ClientEvent::MovePosition {
                        position: p.position,
                        on_ground: p.on_ground,
                    });
                }
            }
            C2sPlayPacket::SetPlayerPositionAndRotation(p) => {
                if self.pending_teleports == 0 {
                    self.position = p.position;
                    self.yaw = p.yaw;
                    self.pitch = p.pitch;

                    self.events.push_back(ClientEvent::MovePositionAndRotation {
                        position: p.position,
                        yaw: p.yaw,
                        pitch: p.pitch,
                        on_ground: p.on_ground,
                    });
                }
            }
            C2sPlayPacket::SetPlayerRotation(p) => {
                if self.pending_teleports == 0 {
                    self.yaw = p.yaw;
                    self.pitch = p.pitch;

                    self.events.push_back(ClientEvent::MoveRotation {
                        yaw: p.yaw,
                        pitch: p.pitch,
                        on_ground: p.on_ground,
                    });
                }
            }
            C2sPlayPacket::SetPlayerOnGround(p) => {
                if self.pending_teleports == 0 {
                    self.events.push_back(ClientEvent::MoveOnGround {
                        on_ground: p.on_ground,
                    });
                }
            }
            C2sPlayPacket::MoveVehicleC2s(p) => {
                if self.pending_teleports == 0 {
                    self.position = p.position;
                    self.yaw = p.yaw;
                    self.pitch = p.pitch;

                    self.events.push_back(ClientEvent::MoveVehicle {
                        position: p.position,
                        yaw: p.yaw,
                        pitch: p.pitch,
                    });
                }
            }
            C2sPlayPacket::PaddleBoat(p) => {
                self.events.push_back(ClientEvent::SteerBoat {
                    left_paddle_turning: p.left_paddle_turning,
                    right_paddle_turning: p.right_paddle_turning,
                });
            }
            C2sPlayPacket::PickItem(_) => {}
            C2sPlayPacket::PlaceRecipe(_) => {}
            C2sPlayPacket::PlayerAbilitiesC2s(_) => {}
            C2sPlayPacket::PlayerAction(p) => {
                if p.sequence.0 != 0 {
                    self.dug_block_sequence = p.sequence.0;
                }

                self.events.push_back(match p.status {
                    play::DiggingStatus::StartedDigging => ClientEvent::Digging {
                        status: DiggingStatus::Start,
                        position: p.location,
                        face: p.face,
                    },
                    play::DiggingStatus::CancelledDigging => ClientEvent::Digging {
                        status: DiggingStatus::Cancel,
                        position: p.location,
                        face: p.face,
                    },
                    play::DiggingStatus::FinishedDigging => ClientEvent::Digging {
                        status: DiggingStatus::Finish,
                        position: p.location,
                        face: p.face,
                    },
                    play::DiggingStatus::DropItemStack => return,
                    play::DiggingStatus::DropItem => ClientEvent::DropItem,
                    play::DiggingStatus::ShootArrowOrFinishEating => return,
                    play::DiggingStatus::SwapItemInHand => return,
                });
            }
            C2sPlayPacket::PlayerCommand(c) => {
                self.events.push_back(match c.action_id {
                    PlayerCommandId::StartSneaking => ClientEvent::StartSneaking,
                    PlayerCommandId::StopSneaking => ClientEvent::StopSneaking,
                    PlayerCommandId::LeaveBed => ClientEvent::LeaveBed,
                    PlayerCommandId::StartSprinting => ClientEvent::StartSprinting,
                    PlayerCommandId::StopSprinting => ClientEvent::StopSprinting,
                    PlayerCommandId::StartJumpWithHorse => ClientEvent::StartJumpWithHorse {
                        jump_boost: c.jump_boost.0 .0 as u8,
                    },
                    PlayerCommandId::StopJumpWithHorse => ClientEvent::StopJumpWithHorse,
                    PlayerCommandId::OpenHorseInventory => ClientEvent::OpenHorseInventory,
                    PlayerCommandId::StartFlyingWithElytra => ClientEvent::StartFlyingWithElytra,
                });
            }
            C2sPlayPacket::PlayerInput(_) => {}
            C2sPlayPacket::PongPlay(_) => {}
            C2sPlayPacket::ChangeRecipeBookSettings(_) => {}
            C2sPlayPacket::SetSeenRecipe(_) => {}
            C2sPlayPacket::RenameItem(_) => {}
            C2sPlayPacket::ResourcePackC2s(p) => self
                .events
                .push_back(ClientEvent::ResourcePackStatusChanged(p)),
            C2sPlayPacket::SeenAdvancements(_) => {}
            C2sPlayPacket::SelectTrade(_) => {}
            C2sPlayPacket::SetBeaconEffect(_) => {}
            C2sPlayPacket::SetHeldItemS2c(e) => {
                self.selected_hotbar_slot = PlayerInventory::hotbar_to_slot(e.slot.0).unwrap();
            }
            C2sPlayPacket::ProgramCommandBlock(_) => {}
            C2sPlayPacket::ProgramCommandBlockMinecart(_) => {}
            C2sPlayPacket::SetCreativeModeSlot(e) => {
                if e.slot == -1 {
                    // The client is trying to drop a stack of items
                    match e.clicked_item {
                        None => log::warn!(
                            "Invalid packet, creative client tried to drop a stack of nothing."
                        ),
                        Some(stack) => self.events.push_back(ClientEvent::DropItemStack { stack }),
                    }
                } else {
                    self.events.push_back(ClientEvent::SetSlotCreative {
                        slot_id: e.slot,
                        slot: e.clicked_item,
                    })
                }
            }
            C2sPlayPacket::ProgramJigsawBlock(_) => {}
            C2sPlayPacket::ProgramStructureBlock(_) => {}
            C2sPlayPacket::UpdateSign(_) => {}
            C2sPlayPacket::SwingArm(p) => self.events.push_back(ClientEvent::ArmSwing(p.hand)),
            C2sPlayPacket::TeleportToEntity(_) => {}
            C2sPlayPacket::UseItemOn(p) => self.events.push_back(ClientEvent::InteractWithBlock {
                hand: p.hand,
                location: p.location,
                face: p.face,
                cursor_pos: p.cursor_pos,
                head_inside_block: p.head_inside_block,
                sequence: p.sequence,
            }),
            C2sPlayPacket::UseItem(_) => {}
        }
    }

    pub(crate) fn update(
        &mut self,
        shared: &SharedServer<C>,
        entities: &Entities<C>,
        worlds: &Worlds<C>,
        player_lists: &PlayerLists<C>,
    ) {
        // Mark the client as disconnected when appropriate.
        if self.recv.is_disconnected() || self.send.as_ref().map_or(true, |s| s.is_disconnected()) {
            self.bits.set_created_this_tick(false);
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
        // so that the user can set the client's initial location, game mode, etc.
        if self.created_this_tick() {
            self.bits.set_spawn(false);

            if let Some(id) = &self.player_list {
                player_lists
                    .get(id)
                    .initial_packets(|p| send_packet(&mut self.send, p));
            }

            let mut dimension_names: Vec<_> = shared
                .dimensions()
                .map(|(id, _)| id.dimension_name())
                .collect();

            dimension_names.push(ident!("{LIBRARY_NAMESPACE}:dummy_dimension"));

            self.send_packet(LoginPlay {
                entity_id: 0, // EntityId 0 is reserved for clients.
                is_hardcore: self.bits.hardcore(),
                gamemode: self.new_game_mode,
                previous_gamemode: self.old_game_mode,
                dimension_names,
                registry_codec: shared.registry_codec().clone(),
                dimension_type_name: world.meta.dimension().dimension_type_name(),
                dimension_name: world.meta.dimension().dimension_name(),
                hashed_seed: 0,
                max_players: VarInt(0),
                view_distance: BoundedInt(VarInt(self.view_distance() as i32)),
                simulation_distance: VarInt(16),
                reduced_debug_info: false,
                enable_respawn_screen: false,
                is_debug: false,
                is_flat: self.bits.flat(),
                last_death_location: self
                    .death_location
                    .map(|(id, pos)| (id.dimension_name(), pos)),
            });

            self.teleport(self.position(), self.yaw(), self.pitch());
        } else {
            if self.bits.spawn() {
                self.bits.set_spawn(false);
                self.loaded_entities.clear();
                self.loaded_chunks.clear();

                // Client bug workaround: send the client to a dummy dimension first.
                // TODO: is there actually a bug?
                self.send_packet(Respawn {
                    dimension_type_name: DimensionId(0).dimension_type_name(),
                    dimension_name: ident!("{LIBRARY_NAMESPACE}:dummy_dimension"),
                    hashed_seed: 0,
                    game_mode: self.game_mode(),
                    previous_game_mode: self.game_mode(),
                    is_debug: false,
                    is_flat: self.bits.flat(),
                    copy_metadata: true,
                    last_death_location: None,
                });

                self.send_packet(Respawn {
                    dimension_type_name: world.meta.dimension().dimension_type_name(),
                    dimension_name: world.meta.dimension().dimension_name(),
                    hashed_seed: 0,
                    game_mode: self.game_mode(),
                    previous_game_mode: self.game_mode(),
                    is_debug: false,
                    is_flat: self.bits.flat(),
                    copy_metadata: true,
                    last_death_location: self
                        .death_location
                        .map(|(id, pos)| (id.dimension_name(), pos)),
                });

                self.teleport(self.position(), self.yaw(), self.pitch());
            }

            // Update game mode
            if self.old_game_mode != self.new_game_mode {
                self.old_game_mode = self.new_game_mode;
                self.send_packet(GameEvent {
                    reason: GameStateChangeReason::ChangeGameMode,
                    value: self.new_game_mode as i32 as f32,
                });
            }

            // If the player list was changed...
            if self.old_player_list != self.player_list {
                // Delete existing entries from old player list.
                if let Some(id) = &self.old_player_list {
                    player_lists
                        .get(id)
                        .clear_packets(|p| send_packet(&mut self.send, p));
                }

                // Get initial packets for new player list.
                if let Some(id) = &self.player_list {
                    player_lists
                        .get(id)
                        .initial_packets(|p| send_packet(&mut self.send, p));
                }

                self.old_player_list = self.player_list.clone();
            } else if let Some(id) = &self.player_list {
                // Update current player list.
                player_lists
                    .get(id)
                    .update_packets(|p| send_packet(&mut self.send, p));
            }
        }

        // Set player attributes
        if self.bits.attack_speed_modified() {
            self.bits.set_attack_speed_modified(false);

            self.send_packet(UpdateAttributes {
                entity_id: VarInt(0),
                properties: vec![EntityAttributesProperty {
                    key: ident!("generic.attack_speed"),
                    value: self.attack_speed,
                    modifiers: Vec::new(),
                }],
            });
        }

        if self.bits.movement_speed_modified() {
            self.bits.set_movement_speed_modified(false);

            self.send_packet(UpdateAttributes {
                entity_id: VarInt(0),
                properties: vec![EntityAttributesProperty {
                    key: ident!("generic.movement_speed"),
                    value: self.movement_speed,
                    modifiers: Vec::new(),
                }],
            });
        }

        // Update the players spawn position (compass position)
        if self.bits.modified_spawn_position() {
            self.bits.set_modified_spawn_position(false);

            self.send_packet(SetDefaultSpawnPosition {
                location: self.spawn_position,
                angle: self.spawn_position_yaw,
            })
        }

        // Update view distance fog on the client.
        if self.bits.view_distance_modified() {
            self.bits.set_view_distance_modified(false);

            if !self.created_this_tick() {
                self.send_packet(SetRenderDistance {
                    view_distance: BoundedInt(VarInt(self.view_distance() as i32)),
                });
            }
        }

        // Check if it's time to send another keepalive.
        if current_tick % (shared.tick_rate() * 8) == 0 {
            if self.bits.got_keepalive() {
                let id = rand::random();
                self.send_packet(KeepAliveS2c { id });
                self.last_keepalive_id = id;
                self.bits.set_got_keepalive(false);
            } else {
                log::warn!(
                    "player {} timed out (no keepalive response)",
                    self.username()
                );
                self.disconnect_no_reason();
            }
        }

        let center = ChunkPos::at(self.position.x, self.position.z);

        // Send the update view position packet if the client changes the chunk they're
        // in.
        if ChunkPos::at(self.old_position.x, self.old_position.z) != center {
            self.send_packet(SetCenterChunk {
                chunk_x: VarInt(center.x),
                chunk_z: VarInt(center.z),
            });
        }

        let dimension = shared.dimension(world.meta.dimension());

        // Update existing chunks and unload those outside the view distance. Chunks
        // that have been overwritten also need to be unloaded.
        self.loaded_chunks.retain(|&pos| {
            // The cache stops chunk data packets from needing to be sent when a player
            // moves to an adjacent chunk and back to the original.
            let cache = 2;

            if let Some(chunk) = world.chunks.get(pos) {
                if is_chunk_in_view_distance(center, pos, self.view_distance + cache)
                    && !chunk.created_this_tick()
                {
                    chunk.block_change_packets(pos, dimension.min_y, |pkt| {
                        send_packet(&mut self.send, pkt)
                    });
                    return true;
                }
            }

            send_packet(
                &mut self.send,
                UnloadChunk {
                    chunk_x: pos.x,
                    chunk_z: pos.z,
                },
            );
            false
        });

        // Load new chunks within the view distance
        for pos in chunks_in_view_distance(center, self.view_distance) {
            if let Some(chunk) = world.chunks.get(pos) {
                if self.loaded_chunks.insert(pos) {
                    self.send_packet(chunk.chunk_data_packet(pos));
                }
            }
        }

        // Acknowledge broken blocks.
        if self.dug_block_sequence != 0 {
            send_packet(
                &mut self.send,
                AcknowledgeBlockChange {
                    sequence: VarInt(self.dug_block_sequence),
                },
            );
            self.dug_block_sequence = 0;
        }

        // Teleport the player.
        //
        // This is done after the chunks are loaded so that the "downloading terrain"
        // screen is closed at the appropriate time.
        if self.bits.teleported_this_tick() {
            self.bits.set_teleported_this_tick(false);

            self.send_packet(SynchronizePlayerPosition {
                position: self.position,
                yaw: self.yaw,
                pitch: self.pitch,
                flags: PlayerPositionLookFlags::new(false, false, false, false, false),
                teleport_id: VarInt(self.teleport_id_counter as i32),
                dismount_vehicle: false,
            });

            self.pending_teleports = self.pending_teleports.wrapping_add(1);

            if self.pending_teleports == 0 {
                log::warn!("too many pending teleports for {}", self.username());
                self.disconnect_no_reason();
                return;
            }

            self.teleport_id_counter = self.teleport_id_counter.wrapping_add(1);
        }

        // Set velocity. Do this after teleporting since teleporting sets velocity to
        // zero.
        if self.bits.velocity_modified() {
            self.bits.set_velocity_modified(false);

            self.send_packet(SetEntityVelocity {
                entity_id: VarInt(0),
                velocity: velocity_to_packet_units(self.velocity),
            });
        }

        // Send chat messages.
        for msg in self.msgs_to_send.drain(..) {
            send_packet(
                &mut self.send,
                SystemChatMessage {
                    chat: msg,
                    kind: VarInt(0),
                },
            );
        }

        // Set action bar.
        if let Some(bar) = self.bar_to_send.take() {
            send_packet(&mut self.send, SetActionBarText { text: bar });
        }

        // Send resource pack prompt.
        if let Some(p) = self.resource_pack_to_send.take() {
            send_packet(&mut self.send, p);
        }

        let mut entities_to_unload = Vec::new();

        // Update all entities that are visible and unload entities that are no
        // longer visible.
        self.loaded_entities.retain(|&id| {
            if let Some(entity) = entities.get(id) {
                debug_assert!(entity.kind() != EntityKind::Marker);
                if self.position.distance(entity.position()) <= self.view_distance as f64 * 16.0 {
                    if let Some(meta) = entity.updated_tracked_data_packet(id) {
                        send_packet(&mut self.send, meta);
                    }

                    let position_delta = entity.position() - entity.old_position();
                    let needs_teleport = position_delta.map(f64::abs).reduce_partial_max() >= 8.0;
                    let flags = entity.bits();

                    if entity.position() != entity.old_position()
                        && !needs_teleport
                        && flags.yaw_or_pitch_modified()
                    {
                        send_packet(
                            &mut self.send,
                            UpdateEntityPositionAndRotation {
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
                                UpdateEntityPosition {
                                    entity_id: VarInt(id.to_network_id()),
                                    delta: (position_delta * 4096.0).as_(),
                                    on_ground: entity.on_ground(),
                                },
                            );
                        }

                        if flags.yaw_or_pitch_modified() {
                            send_packet(
                                &mut self.send,
                                UpdateEntityRotation {
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
                            SetEntityVelocity {
                                entity_id: VarInt(id.to_network_id()),
                                velocity: velocity_to_packet_units(entity.velocity()),
                            },
                        );
                    }

                    if flags.head_yaw_modified() {
                        send_packet(
                            &mut self.send,
                            SetHeadRotation {
                                entity_id: VarInt(id.to_network_id()),
                                head_yaw: ByteAngle::from_degrees(entity.head_yaw()),
                            },
                        )
                    }

                    send_entity_events(&mut self.send, id.to_network_id(), entity.events());

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
        self.player_data.updated_tracked_data(&mut data);

        if !data.is_empty() {
            data.push(0xff);

            self.send_packet(SetEntityMetadata {
                entity_id: VarInt(0),
                metadata: RawBytes(data),
            });
        }

        // Spawn new entities within the view distance.
        let pos = self.position();
        let view_dist = self.view_distance;
        world.spatial_index.query::<_, _, ()>(
            |bb| bb.projected_point(pos).distance(pos) <= view_dist as f64 * 16.0,
            |id, _| {
                let entity = entities
                    .get(id)
                    .expect("entity IDs in spatial index should be valid at this point");

                // Skip spawning players not in the player list because they would be invisible
                // otherwise.
                if entity.kind() == EntityKind::Player {
                    if let Some(list_id) = &self.player_list {
                        player_lists.get(list_id).entry(entity.uuid())?;
                    } else {
                        return None;
                    }
                }

                if entity.kind() != EntityKind::Marker
                    && entity.uuid() != self.uuid
                    && self.loaded_entities.insert(id)
                {
                    entity.spawn_packets(id, |pkt| self.send_packet(pkt));

                    if let Some(meta) = entity.initial_tracked_data_packet(id) {
                        self.send_packet(meta);
                    }

                    send_entity_events(&mut self.send, id.to_network_id(), entity.events());
                }
                None
            },
        );

        send_entity_events(&mut self.send, 0, &self.entity_events);
        self.entity_events.clear();

        self.player_data.clear_modifications();
        self.old_position = self.position;
        self.bits.set_created_this_tick(false);

        send_packet(&mut self.send, S2cPlayMessage::Flush);
    }
}

type SendOpt = Option<Sender<S2cPlayMessage>>;

fn send_packet(send_opt: &mut SendOpt, pkt: impl Into<S2cPlayMessage>) {
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

fn send_entity_events(send_opt: &mut SendOpt, entity_id: i32, events: &[entity::EntityEvent]) {
    for &event in events {
        match event.status_or_animation() {
            StatusOrAnimation::Status(code) => send_packet(
                send_opt,
                EntityEvent {
                    entity_id,
                    entity_status: code,
                },
            ),
            StatusOrAnimation::Animation(code) => send_packet(
                send_opt,
                EntityAnimationS2c {
                    entity_id: VarInt(entity_id),
                    animation: code,
                },
            ),
        }
    }
}
