//! Dynamic actors in a world.

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::iter::FusedIterator;
use std::num::NonZeroU32;

use bitfield_struct::bitfield;
pub use kinds::{EntityEnum, EntityKind};
use rayon::iter::ParallelIterator;
use uuid::Uuid;
use vek::{Aabb, Vec3};

use crate::config::Config;
use crate::protocol_inner::packets::play::s2c::{
    EntitySpawn, ExperienceOrbSpawn, PlayerSpawn, S2cPlayPacket, EntityTrackerUpdate,
};
use crate::protocol_inner::{ByteAngle, RawBytes, VarInt};
use crate::slotmap::{Key, SlotMap};
use crate::util::aabb_from_bottom_and_size;
use crate::world::WorldId;
use crate::STANDARD_TPS;

pub mod data;
pub mod kinds;

include!(concat!(env!("OUT_DIR"), "/entity_event.rs"));

/// A container for all [`Entity`]s on a [`Server`](crate::server::Server).
///
/// # Spawning Player Entities
///
/// [`Player`] entities are treated specially by the client. For the player
/// entity to be visible to clients, the player's UUID must be added to the
/// [`PlayerList`] _before_ being loaded by the client.
///
/// [`Player`]: crate::entity::types::Player
/// [`PlayerList`]: crate::player_list::PlayerList
pub struct Entities<C: Config> {
    sm: SlotMap<Entity<C>>,
    uuid_to_entity: HashMap<Uuid, EntityId>,
    network_id_to_entity: HashMap<NonZeroU32, u32>,
}

impl<C: Config> Entities<C> {
    pub(crate) fn new() -> Self {
        Self {
            sm: SlotMap::new(),
            uuid_to_entity: HashMap::new(),
            network_id_to_entity: HashMap::new(),
        }
    }

    /// Spawns a new entity with a random UUID. A reference to the entity along
    /// with its ID is returned.
    pub fn create(&mut self, kind: EntityKind, data: C::EntityState) -> (EntityId, &mut Entity<C>) {
        self.create_with_uuid(kind, Uuid::from_bytes(rand::random()), data)
            .expect("UUID collision")
    }

    /// Like [`Self::create`], but requires specifying the new
    /// entity's UUID.
    ///
    /// The provided UUID must not conflict with an existing entity UUID. If it
    /// does, `None` is returned and the entity is not spawned.
    pub fn create_with_uuid(
        &mut self,
        kind: EntityKind,
        uuid: Uuid,
        data: C::EntityState,
    ) -> Option<(EntityId, &mut Entity<C>)> {
        match self.uuid_to_entity.entry(uuid) {
            Entry::Occupied(_) => None,
            Entry::Vacant(ve) => {
                let (k, e) = self.sm.insert(Entity {
                    state: data,
                    variants: EntityEnum::new(kind),
                    events: Vec::new(),
                    flags: EntityFlags(0),
                    world: WorldId::NULL,
                    new_position: Vec3::default(),
                    old_position: Vec3::default(),
                    yaw: 0.0,
                    pitch: 0.0,
                    head_yaw: 0.0,
                    velocity: Vec3::default(),
                    uuid,
                });

                // TODO check for overflowing version?
                self.network_id_to_entity.insert(k.version(), k.index());

                ve.insert(EntityId(k));

                Some((EntityId(k), e))
            }
        }
    }

    /// Removes an entity from the server.
    ///
    /// If the given entity ID is valid, `true` is returned and the entity is
    /// deleted. Otherwise, `false` is returned and the function has no effect.
    pub fn delete(&mut self, entity: EntityId) -> bool {
        if let Some(e) = self.sm.remove(entity.0) {
            self.uuid_to_entity
                .remove(&e.uuid)
                .expect("UUID should have been in UUID map");

            self.network_id_to_entity
                .remove(&entity.0.version())
                .expect("network ID should have been in the network ID map");

            true
        } else {
            false
        }
    }

    /// Removes all entities from the server for which `f` returns `true`.
    ///
    /// All entities are visited in an unspecified order.
    pub fn retain(&mut self, mut f: impl FnMut(EntityId, &mut Entity<C>) -> bool) {
        self.sm.retain(|k, v| {
            if f(EntityId(k), v) {
                true
            } else {
                self.uuid_to_entity
                    .remove(&v.uuid)
                    .expect("UUID should have been in UUID map");

                self.network_id_to_entity
                    .remove(&k.version())
                    .expect("network ID should have been in the network ID map");

                false
            }
        });
    }

    /// Returns the number of entities in this container.
    pub fn count(&self) -> usize {
        self.sm.len()
    }

    /// Gets the [`EntityId`] of the entity with the given UUID in an efficient
    /// manner.
    ///
    /// If there is no entity with the UUID, `None` is returned.
    pub fn get_with_uuid(&self, uuid: Uuid) -> Option<EntityId> {
        self.uuid_to_entity.get(&uuid).cloned()
    }

    /// Gets a shared reference to the entity with the given [`EntityId`].
    ///
    /// If the ID is invalid, `None` is returned.
    pub fn get(&self, entity: EntityId) -> Option<&Entity<C>> {
        self.sm.get(entity.0)
    }

    /// Gets an exclusive reference to the entity with the given [`EntityId`].
    ///
    /// If the ID is invalid, `None` is returned.
    pub fn get_mut(&mut self, entity: EntityId) -> Option<&mut Entity<C>> {
        self.sm.get_mut(entity.0)
    }

    pub(crate) fn get_with_network_id(&self, network_id: i32) -> Option<EntityId> {
        let version = NonZeroU32::new(network_id as u32)?;
        let index = *self.network_id_to_entity.get(&version)?;
        Some(EntityId(Key::new(index, version)))
    }

    /// Returns an immutable iterator over all entities on the server in an
    /// unspecified order.
    pub fn iter(&self) -> impl FusedIterator<Item = (EntityId, &Entity<C>)> + Clone + '_ {
        self.sm.iter().map(|(k, v)| (EntityId(k), v))
    }

    /// Returns a mutable iterator over all entities on the server in an
    /// unspecified order.
    pub fn iter_mut(&mut self) -> impl FusedIterator<Item = (EntityId, &mut Entity<C>)> + '_ {
        self.sm.iter_mut().map(|(k, v)| (EntityId(k), v))
    }

    /// Returns a parallel immutable iterator over all entities on the server in
    /// an unspecified order.
    pub fn par_iter(&self) -> impl ParallelIterator<Item = (EntityId, &Entity<C>)> + Clone + '_ {
        self.sm.par_iter().map(|(k, v)| (EntityId(k), v))
    }

    /// Returns a parallel mutable iterator over all clients on the server in an
    /// unspecified order.
    pub fn par_iter_mut(
        &mut self,
    ) -> impl ParallelIterator<Item = (EntityId, &mut Entity<C>)> + '_ {
        self.sm.par_iter_mut().map(|(k, v)| (EntityId(k), v))
    }

    pub(crate) fn update(&mut self) {
        for (_, e) in self.iter_mut() {
            e.old_position = e.new_position;
            e.variants.clear_modifications();
            e.events.clear();

            e.flags.set_yaw_or_pitch_modified(false);
            e.flags.set_head_yaw_modified(false);
            e.flags.set_velocity_modified(false);
        }
    }
}

/// An identifier for an [`Entity`] on the server.
///
/// Entity IDs are either _valid_ or _invalid_. Valid entity IDs point to
/// entities that have not been deleted, while invalid IDs point to those that
/// have. Once an ID becomes invalid, it will never become valid again.
///
/// The [`Ord`] instance on this type is correct but otherwise unspecified. This
/// is useful for storing IDs in containers such as
/// [`BTreeMap`](std::collections::BTreeMap).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub struct EntityId(Key);

impl EntityId {
    /// The value of the default entity ID which is always invalid.
    pub const NULL: Self = Self(Key::NULL);

    pub(crate) fn to_network_id(self) -> i32 {
        self.0.version().get() as i32
    }
}

/// Represents an entity on the server.
///
/// In essence, an entity is anything in a world that isn't a block or client.
/// Entities include paintings, falling blocks, zombies, fireballs, and more.
///
/// Every entity has common state which is accessible directly from
/// this struct. This includes position, rotation, velocity, UUID, and hitbox.
/// To access data that is not common to every kind of entity, see
/// [`Self::data`].
pub struct Entity<C: Config> {
    /// Custom data.
    pub state: C::EntityState,
    variants: EntityEnum,
    flags: EntityFlags,
    events: Vec<Event>,
    world: WorldId,
    new_position: Vec3<f64>,
    old_position: Vec3<f64>,
    yaw: f32,
    pitch: f32,
    head_yaw: f32,
    velocity: Vec3<f32>,
    uuid: Uuid,
}

#[bitfield(u8)]
pub(crate) struct EntityFlags {
    pub yaw_or_pitch_modified: bool,
    pub head_yaw_modified: bool,
    pub velocity_modified: bool,
    pub on_ground: bool,
    #[bits(4)]
    _pad: u8,
}

impl<C: Config> Entity<C> {
    pub(crate) fn flags(&self) -> EntityFlags {
        self.flags
    }

    pub fn view(&self) -> &EntityEnum {
        &self.variants
    }

    pub fn view_mut(&mut self) -> &mut EntityEnum {
        &mut self.variants
    }

    /// Gets the [`EntityKind`] of this entity.
    pub fn kind(&self) -> EntityKind {
        self.variants.kind()
    }

    pub fn trigger_event(&mut self, event: Event) {
        self.events.push(event);
    }

    pub(crate) fn events(&self) -> &[Event] {
        &self.events
    }

    /// Gets the [`WorldId`](crate::world::WorldId) of the world this entity is
    /// located in.
    ///
    /// By default, entities are located in
    /// [`WorldId::NULL`](crate::world::WorldId::NULL).
    pub fn world(&self) -> WorldId {
        self.world
    }

    /// Sets the world this entity is located in.
    pub fn set_world(&mut self, world: WorldId) {
        self.world = world;
    }

    /// Gets the position of this entity in the world it inhabits.
    ///
    /// The position of an entity is located on the botton of its
    /// hitbox and not the center.
    pub fn position(&self) -> Vec3<f64> {
        self.new_position
    }

    /// Sets the position of this entity in the world it inhabits.
    ///
    /// The position of an entity is located on the botton of its
    /// hitbox and not the center.
    pub fn set_position(&mut self, pos: impl Into<Vec3<f64>>) {
        self.new_position = pos.into();
    }

    /// Returns the position of this entity as it existed at the end of the
    /// previous tick.
    pub(crate) fn old_position(&self) -> Vec3<f64> {
        self.old_position
    }

    /// Gets the yaw of this entity in degrees.
    pub fn yaw(&self) -> f32 {
        self.yaw
    }

    /// Sets the yaw of this entity in degrees.
    pub fn set_yaw(&mut self, yaw: f32) {
        if self.yaw != yaw {
            self.yaw = yaw;
            self.flags.set_yaw_or_pitch_modified(true);
        }
    }

    /// Gets the pitch of this entity in degrees.
    pub fn pitch(&self) -> f32 {
        self.pitch
    }

    /// Sets the pitch of this entity in degrees.
    pub fn set_pitch(&mut self, pitch: f32) {
        if self.pitch != pitch {
            self.pitch = pitch;
            self.flags.set_yaw_or_pitch_modified(true);
        }
    }

    /// Gets the head yaw of this entity in degrees.
    pub fn head_yaw(&self) -> f32 {
        self.head_yaw
    }

    /// Sets the head yaw of this entity in degrees.
    pub fn set_head_yaw(&mut self, head_yaw: f32) {
        if self.head_yaw != head_yaw {
            self.head_yaw = head_yaw;
            self.flags.set_head_yaw_modified(true);
        }
    }

    /// Gets the velocity of this entity in meters per second.
    pub fn velocity(&self) -> Vec3<f32> {
        self.velocity
    }

    /// Sets the velocity of this entity in meters per second.
    pub fn set_velocity(&mut self, velocity: impl Into<Vec3<f32>>) {
        let new_vel = velocity.into();

        if self.velocity != new_vel {
            self.velocity = new_vel;
            self.flags.set_velocity_modified(true);
        }
    }

    /// Gets the value of the "on ground" flag.
    pub fn on_ground(&self) -> bool {
        self.flags.on_ground()
    }

    /// Sets the value of the "on ground" flag.
    pub fn set_on_ground(&mut self, on_ground: bool) {
        self.flags.set_on_ground(on_ground);
    }

    /// Gets the UUID of this entity.
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    /// Returns the hitbox of this entity.
    ///
    /// The hitbox describes the space that an entity occupies. Clients interact
    /// with this space to create an [interact event].
    ///
    /// The hitbox of an entity is determined by its position, entity type, and
    /// other state specific to that type.
    ///
    /// [interact event]: crate::client::Event::InteractWithEntity
    pub fn hitbox(&self) -> Aabb<f64> {
        let dims = match &self.variants {
            EntityEnum::Allay(_) => [0.6, 0.35, 0.6],
            EntityEnum::ChestBoat(_) => [1.375, 0.5625, 1.375],
            EntityEnum::Frog(_) => [0.5, 0.5, 0.5],
            EntityEnum::Tadpole(_) => [0.4, 0.3, 0.4],
            EntityEnum::Warden(_) => [0.9, 2.9, 0.9],
            EntityEnum::AreaEffectCloud(e) => [
                e.get_radius() as f64 * 2.0,
                0.5,
                e.get_radius() as f64 * 2.0,
            ],
            EntityEnum::ArmorStand(e) => {
                if e.get_marker() {
                    [0.0, 0.0, 0.0]
                } else if e.get_small() {
                    [0.5, 0.9875, 0.5]
                } else {
                    [0.5, 1.975, 0.5]
                }
            }
            EntityEnum::Arrow(_) => [0.5, 0.5, 0.5],
            EntityEnum::Axolotl(_) => [1.3, 0.6, 1.3],
            EntityEnum::Bat(_) => [0.5, 0.9, 0.5],
            EntityEnum::Bee(_) => [0.7, 0.6, 0.7], // TODO: baby size?
            EntityEnum::Blaze(_) => [0.6, 1.8, 0.6],
            EntityEnum::Boat(_) => [1.375, 0.5625, 1.375],
            EntityEnum::Cat(_) => [0.6, 0.7, 0.6],
            EntityEnum::CaveSpider(_) => [0.7, 0.5, 0.7],
            EntityEnum::Chicken(_) => [0.4, 0.7, 0.4], // TODO: baby size?
            EntityEnum::Cod(_) => [0.5, 0.3, 0.5],
            EntityEnum::Cow(_) => [0.9, 1.4, 0.9], // TODO: baby size?
            EntityEnum::Creeper(_) => [0.6, 1.7, 0.6],
            EntityEnum::Dolphin(_) => [0.9, 0.6, 0.9],
            EntityEnum::Donkey(_) => [1.5, 1.39648, 1.5], // TODO: baby size?
            EntityEnum::DragonFireball(_) => [1.0, 1.0, 1.0],
            EntityEnum::Drowned(_) => [0.6, 1.95, 0.6], // TODO: baby size?
            EntityEnum::ElderGuardian(_) => [1.9975, 1.9975, 1.9975],
            EntityEnum::EndCrystal(_) => [2.0, 2.0, 2.0],
            EntityEnum::EnderDragon(_) => [16.0, 8.0, 16.0],
            EntityEnum::Enderman(_) => [0.6, 2.9, 0.6],
            EntityEnum::Endermite(_) => [0.4, 0.3, 0.4],
            EntityEnum::Evoker(_) => [0.6, 1.95, 0.6],
            EntityEnum::EvokerFangs(_) => [0.5, 0.8, 0.5],
            EntityEnum::ExperienceOrb(_) => [0.5, 0.5, 0.5],
            EntityEnum::EyeOfEnder(_) => [0.25, 0.25, 0.25],
            EntityEnum::FallingBlock(_) => [0.98, 0.98, 0.98],
            EntityEnum::FireworkRocket(_) => [0.25, 0.25, 0.25],
            EntityEnum::Fox(_) => [0.6, 0.7, 0.6], // TODO: baby size?
            EntityEnum::Ghast(_) => [4.0, 4.0, 4.0],
            EntityEnum::Giant(_) => [3.6, 12.0, 3.6],
            EntityEnum::GlowItemFrame(_) => todo!("account for rotation"),
            EntityEnum::GlowSquid(_) => [0.8, 0.8, 0.8],
            EntityEnum::Goat(_) => [1.3, 0.9, 1.3], // TODO: baby size?
            EntityEnum::Guardian(_) => [0.85, 0.85, 0.85],
            EntityEnum::Hoglin(_) => [1.39648, 1.4, 1.39648], // TODO: baby size?
            EntityEnum::Horse(_) => [1.39648, 1.6, 1.39648],  // TODO: baby size?
            EntityEnum::Husk(_) => [0.6, 1.95, 0.6],          // TODO: baby size?
            EntityEnum::Illusioner(_) => [0.6, 1.95, 0.6],
            EntityEnum::IronGolem(_) => [1.4, 2.7, 1.4],
            EntityEnum::Item(_) => [0.25, 0.25, 0.25],
            EntityEnum::ItemFrame(_) => todo!("account for rotation"),
            EntityEnum::Fireball(_) => [1.0, 1.0, 1.0],
            EntityEnum::LeashKnot(_) => [0.375, 0.5, 0.375],
            EntityEnum::Lightning(_) => [0.0, 0.0, 0.0],
            EntityEnum::Llama(_) => [0.9, 1.87, 0.9], // TODO: baby size?
            EntityEnum::LlamaSpit(_) => [0.25, 0.25, 0.25],
            EntityEnum::MagmaCube(e) => {
                let s = e.get_slime_size() as f64 * 0.51000005;
                [s, s, s]
            }
            EntityEnum::Marker(_) => [0.0, 0.0, 0.0],
            EntityEnum::Minecart(_) => [0.98, 0.7, 0.98],
            EntityEnum::ChestMinecart(_) => [0.98, 0.7, 0.98],
            EntityEnum::CommandBlockMinecart(_) => [0.98, 0.7, 0.98],
            EntityEnum::FurnaceMinecart(_) => [0.98, 0.7, 0.98],
            EntityEnum::HopperMinecart(_) => [0.98, 0.7, 0.98],
            EntityEnum::SpawnerMinecart(_) => [0.98, 0.7, 0.98],
            EntityEnum::TntMinecart(_) => [0.98, 0.7, 0.98],
            EntityEnum::Mule(_) => [1.39648, 1.6, 1.39648], // TODO: baby size?
            EntityEnum::Mooshroom(_) => [0.9, 1.4, 0.9],    // TODO: baby size?
            EntityEnum::Ocelot(_) => [0.6, 0.7, 0.6],       // TODO: baby size?
            EntityEnum::Painting(_) => todo!("account for rotation and type"),
            EntityEnum::Panda(_) => [0.6, 0.7, 0.6], // TODO: baby size?
            EntityEnum::Parrot(_) => [0.5, 0.9, 0.5],
            EntityEnum::Phantom(_) => [0.9, 0.5, 0.9],
            EntityEnum::Pig(_) => [0.9, 0.9, 0.9], // TODO: baby size?
            EntityEnum::Piglin(_) => [0.6, 1.95, 0.6], // TODO: baby size?
            EntityEnum::PiglinBrute(_) => [0.6, 1.95, 0.6],
            EntityEnum::Pillager(_) => [0.6, 1.95, 0.6],
            EntityEnum::PolarBear(_) => [1.4, 1.4, 1.4], // TODO: baby size?
            EntityEnum::Tnt(_) => [0.98, 0.98, 0.98],
            EntityEnum::Pufferfish(_) => [0.7, 0.7, 0.7],
            EntityEnum::Rabbit(_) => [0.4, 0.5, 0.4], // TODO: baby size?
            EntityEnum::Ravager(_) => [1.95, 2.2, 1.95],
            EntityEnum::Salmon(_) => [0.7, 0.4, 0.7],
            EntityEnum::Sheep(_) => [0.9, 1.3, 0.9], // TODO: baby size?
            EntityEnum::Shulker(_) => [1.0, 1.0, 1.0], // TODO: how is height calculated?
            EntityEnum::ShulkerBullet(_) => [0.3125, 0.3125, 0.3125],
            EntityEnum::Silverfish(_) => [0.4, 0.3, 0.4],
            EntityEnum::Skeleton(_) => [0.6, 1.99, 0.6],
            EntityEnum::SkeletonHorse(_) => [1.39648, 1.6, 1.39648], // TODO: baby size?
            EntityEnum::Slime(e) => {
                let s = 0.51000005 * e.get_slime_size() as f64;
                [s, s, s]
            }
            EntityEnum::SmallFireball(_) => [0.3125, 0.3125, 0.3125],
            EntityEnum::SnowGolem(_) => [0.7, 1.9, 0.7],
            EntityEnum::Snowball(_) => [0.25, 0.25, 0.25],
            EntityEnum::SpectralArrow(_) => [0.5, 0.5, 0.5],
            EntityEnum::Spider(_) => [1.4, 0.9, 1.4],
            EntityEnum::Squid(_) => [0.8, 0.8, 0.8],
            EntityEnum::Stray(_) => [0.6, 1.99, 0.6],
            EntityEnum::Strider(_) => [0.9, 1.7, 0.9], // TODO: baby size?
            EntityEnum::Egg(_) => [0.25, 0.25, 0.25],
            EntityEnum::EnderPearl(_) => [0.25, 0.25, 0.25],
            EntityEnum::ExperienceBottle(_) => [0.25, 0.25, 0.25],
            EntityEnum::Potion(_) => [0.25, 0.25, 0.25],
            EntityEnum::Trident(_) => [0.5, 0.5, 0.5],
            EntityEnum::TraderLlama(_) => [0.9, 1.87, 0.9],
            EntityEnum::TropicalFish(_) => [0.5, 0.4, 0.5],
            EntityEnum::Turtle(_) => [1.2, 0.4, 1.2], // TODO: baby size?
            EntityEnum::Vex(_) => [0.4, 0.8, 0.4],
            EntityEnum::Villager(_) => [0.6, 1.95, 0.6], // TODO: baby size?
            EntityEnum::Vindicator(_) => [0.6, 1.95, 0.6],
            EntityEnum::WanderingTrader(_) => [0.6, 1.95, 0.6],
            EntityEnum::Witch(_) => [0.6, 1.95, 0.6],
            EntityEnum::Wither(_) => [0.9, 3.5, 0.9],
            EntityEnum::WitherSkeleton(_) => [0.7, 2.4, 0.7],
            EntityEnum::WitherSkull(_) => [0.3125, 0.3125, 0.3125],
            EntityEnum::Wolf(_) => [0.6, 0.85, 0.6], // TODO: baby size?
            EntityEnum::Zoglin(_) => [1.39648, 1.4, 1.39648], // TODO: baby size?
            EntityEnum::Zombie(_) => [0.6, 1.95, 0.6], // TODO: baby size?
            EntityEnum::ZombieHorse(_) => [1.39648, 1.6, 1.39648], // TODO: baby size?
            EntityEnum::ZombieVillager(_) => [0.6, 1.95, 0.6], // TODO: baby size?
            EntityEnum::ZombifiedPiglin(_) => [0.6, 1.95, 0.6], // TODO: baby size?
            EntityEnum::Player(_) => [0.6, 1.8, 0.6], // TODO: changes depending on the pose.
            EntityEnum::FishingBobber(_) => [0.25, 0.25, 0.25],
        };

        aabb_from_bottom_and_size(self.new_position, dims.into())
    }

    /// Gets the tracked data packet to send to clients after this entity has
    /// been spawned.
    ///
    /// Returns `None` if all the tracked data is at its default values.
    pub(crate) fn initial_tracked_data_packet(
        &self,
        this_id: EntityId,
    ) -> Option<EntityTrackerUpdate> {
        self.variants
            .initial_tracked_data()
            .map(|meta| EntityTrackerUpdate {
                entity_id: VarInt(this_id.to_network_id()),
                metadata: RawBytes(meta),
            })
    }

    /// Gets the tracked data packet to send to clients when the entity is
    /// modified.
    ///
    /// Returns `None` if this entity's tracked data has not been modified.
    pub(crate) fn updated_tracked_data_packet(
        &self,
        this_id: EntityId,
    ) -> Option<EntityTrackerUpdate> {
        self.variants
            .updated_tracked_data()
            .map(|meta| EntityTrackerUpdate {
                entity_id: VarInt(this_id.to_network_id()),
                metadata: RawBytes(meta),
            })
    }

    pub(crate) fn spawn_packet(&self, this_id: EntityId) -> Option<EntitySpawnPacket> {
        match &self.variants {
            EntityEnum::Marker(_) => None,
            EntityEnum::ExperienceOrb(_) => {
                Some(EntitySpawnPacket::ExperienceOrb(ExperienceOrbSpawn {
                    entity_id: VarInt(this_id.to_network_id()),
                    position: self.new_position,
                    count: 0, // TODO
                }))
            }
            EntityEnum::Player(_) => Some(EntitySpawnPacket::Player(PlayerSpawn {
                entity_id: VarInt(this_id.to_network_id()),
                player_uuid: self.uuid,
                position: self.new_position,
                yaw: ByteAngle::from_degrees(self.yaw),
                pitch: ByteAngle::from_degrees(self.pitch),
            })),
            _ => Some(EntitySpawnPacket::Entity(EntitySpawn {
                entity_id: VarInt(this_id.to_network_id()),
                object_uuid: self.uuid,
                kind: VarInt(self.kind() as i32),
                position: self.new_position,
                pitch: ByteAngle::from_degrees(self.pitch),
                yaw: ByteAngle::from_degrees(self.yaw),
                head_yaw: ByteAngle::from_degrees(self.head_yaw),
                data: VarInt(1), // TODO
                velocity: velocity_to_packet_units(self.velocity),
            })),
        }
    }
}

pub(crate) fn velocity_to_packet_units(vel: Vec3<f32>) -> Vec3<i16> {
    // The saturating cast to i16 is desirable.
    (8000.0 / STANDARD_TPS as f32 * vel).as_()
}

pub(crate) enum EntitySpawnPacket {
    Entity(EntitySpawn),
    ExperienceOrb(ExperienceOrbSpawn),
    Player(PlayerSpawn),
}

impl From<EntitySpawnPacket> for S2cPlayPacket {
    fn from(pkt: EntitySpawnPacket) -> Self {
        match pkt {
            EntitySpawnPacket::Entity(pkt) => pkt.into(),
            EntitySpawnPacket::ExperienceOrb(pkt) => pkt.into(),
            EntitySpawnPacket::Player(pkt) => pkt.into(),
        }
    }
}
