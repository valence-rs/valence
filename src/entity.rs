//! Entities in a world.

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::iter::FusedIterator;
use std::num::NonZeroU32;

use bitfield_struct::bitfield;
pub use data::{EntityKind, TrackedData};
use rayon::iter::ParallelIterator;
use uuid::Uuid;
use vek::{Aabb, Vec3};

use crate::config::Config;
use crate::protocol::packets::s2c::play::{
    S2cPlayPacket, SetEntityMetadata, SpawnEntity, SpawnExperienceOrb, SpawnPlayer,
};
use crate::protocol::{ByteAngle, RawBytes, VarInt};
use crate::slab_versioned::{Key, VersionedSlab};
use crate::util::aabb_from_bottom_and_size;
use crate::world::WorldId;
use crate::STANDARD_TPS;

pub mod data;
pub mod types;

include!(concat!(env!("OUT_DIR"), "/entity_event.rs"));

/// A container for all [`Entity`]s on a server.
///
/// # Spawning Player Entities
///
/// [`Player`] entities are treated specially by the client. For the player
/// entity to be visible to clients, the player's UUID must be added to the
/// [`PlayerList`] _before_ being loaded by the client.
///
/// [`Player`]: crate::entity::data::Player
/// [`PlayerList`]: crate::player_list::PlayerList
pub struct Entities<C: Config> {
    slab: VersionedSlab<Entity<C>>,
    uuid_to_entity: HashMap<Uuid, EntityId>,
    network_id_to_entity: HashMap<NonZeroU32, u32>,
}

impl<C: Config> Entities<C> {
    pub(crate) fn new() -> Self {
        Self {
            slab: VersionedSlab::new(),
            uuid_to_entity: HashMap::new(),
            network_id_to_entity: HashMap::new(),
        }
    }

    /// Spawns a new entity with a random UUID. A reference to the entity along
    /// with its ID is returned.
    pub fn insert(
        &mut self,
        kind: EntityKind,
        state: C::EntityState,
    ) -> (EntityId, &mut Entity<C>) {
        self.insert_with_uuid(kind, Uuid::from_bytes(rand::random()), state)
            .expect("UUID collision")
    }

    /// Like [`Self::insert`], but requires specifying the new
    /// entity's UUID.
    ///
    /// The provided UUID must not conflict with an existing entity UUID. If it
    /// does, `None` is returned and the entity is not spawned.
    pub fn insert_with_uuid(
        &mut self,
        kind: EntityKind,
        uuid: Uuid,
        data: C::EntityState,
    ) -> Option<(EntityId, &mut Entity<C>)> {
        match self.uuid_to_entity.entry(uuid) {
            Entry::Occupied(_) => None,
            Entry::Vacant(ve) => {
                let (k, e) = self.slab.insert(Entity {
                    state: data,
                    variants: TrackedData::new(kind),
                    events: Vec::new(),
                    bits: EntityBits::new(),
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
    /// If the given entity ID is valid, the entity's `EntityState` is returned
    /// and the entity is deleted. Otherwise, `None` is returned and the
    /// function has no effect.
    pub fn remove(&mut self, entity: EntityId) -> Option<C::EntityState> {
        self.slab.remove(entity.0).map(|e| {
            self.uuid_to_entity
                .remove(&e.uuid)
                .expect("UUID should have been in UUID map");

            self.network_id_to_entity
                .remove(&entity.0.version())
                .expect("network ID should have been in the network ID map");

            e.state
        })
    }

    /// Removes all entities from the server for which `f` returns `true`.
    ///
    /// All entities are visited in an unspecified order.
    pub fn retain(&mut self, mut f: impl FnMut(EntityId, &mut Entity<C>) -> bool) {
        self.slab.retain(|k, v| {
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
    pub fn len(&self) -> usize {
        self.slab.len()
    }

    /// Returns `true` if there are no entities.
    pub fn is_empty(&self) -> bool {
        self.slab.len() == 0
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
        self.slab.get(entity.0)
    }

    /// Gets an exclusive reference to the entity with the given [`EntityId`].
    ///
    /// If the ID is invalid, `None` is returned.
    pub fn get_mut(&mut self, entity: EntityId) -> Option<&mut Entity<C>> {
        self.slab.get_mut(entity.0)
    }

    pub(crate) fn get_with_network_id(&self, network_id: i32) -> Option<EntityId> {
        let version = NonZeroU32::new(network_id as u32)?;
        let index = *self.network_id_to_entity.get(&version)?;
        Some(EntityId(Key::new(index, version)))
    }

    /// Returns an iterator over all entities on the server in an unspecified
    /// order.
    pub fn iter(
        &self,
    ) -> impl ExactSizeIterator<Item = (EntityId, &Entity<C>)> + FusedIterator + Clone + '_ {
        self.slab.iter().map(|(k, v)| (EntityId(k), v))
    }

    /// Returns a mutable iterator over all entities on the server in an
    /// unspecified order.
    pub fn iter_mut(
        &mut self,
    ) -> impl ExactSizeIterator<Item = (EntityId, &mut Entity<C>)> + FusedIterator + '_ {
        self.slab.iter_mut().map(|(k, v)| (EntityId(k), v))
    }

    /// Returns a parallel iterator over all entities on the server in an
    /// unspecified order.
    pub fn par_iter(&self) -> impl ParallelIterator<Item = (EntityId, &Entity<C>)> + Clone + '_ {
        self.slab.par_iter().map(|(k, v)| (EntityId(k), v))
    }

    /// Returns a parallel mutable iterator over all clients on the server in an
    /// unspecified order.
    pub fn par_iter_mut(
        &mut self,
    ) -> impl ParallelIterator<Item = (EntityId, &mut Entity<C>)> + '_ {
        self.slab.par_iter_mut().map(|(k, v)| (EntityId(k), v))
    }

    pub(crate) fn update(&mut self) {
        for (_, e) in self.iter_mut() {
            e.old_position = e.new_position;
            e.variants.clear_modifications();
            e.events.clear();

            e.bits.set_yaw_or_pitch_modified(false);
            e.bits.set_head_yaw_modified(false);
            e.bits.set_velocity_modified(false);
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
/// An entity is mostly anything in a world that isn't a block or client.
/// Entities include paintings, falling blocks, zombies, fireballs, and more.
///
/// Every entity has common state which is accessible directly from
/// this struct. This includes position, rotation, velocity, UUID, and hitbox.
/// To access data that is not common to every kind of entity, see
/// [`Self::data`].
pub struct Entity<C: Config> {
    /// Custom data.
    pub state: C::EntityState,
    variants: TrackedData,
    bits: EntityBits,
    events: Vec<EntityEvent>,
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
pub(crate) struct EntityBits {
    pub yaw_or_pitch_modified: bool,
    pub head_yaw_modified: bool,
    pub velocity_modified: bool,
    pub on_ground: bool,
    #[bits(4)]
    _pad: u8,
}

impl<C: Config> Entity<C> {
    pub(crate) fn bits(&self) -> EntityBits {
        self.bits
    }

    /// Returns a shared reference to this entity's tracked data.
    pub fn data(&self) -> &TrackedData {
        &self.variants
    }

    /// Returns an exclusive reference to this entity's tracked data.
    pub fn data_mut(&mut self) -> &mut TrackedData {
        &mut self.variants
    }

    /// Gets the [`EntityKind`] of this entity.
    pub fn kind(&self) -> EntityKind {
        self.variants.kind()
    }

    /// Triggers an entity event for this entity.
    pub fn push_event(&mut self, event: EntityEvent) {
        self.events.push(event);
    }

    pub(crate) fn events(&self) -> &[EntityEvent] {
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
            self.bits.set_yaw_or_pitch_modified(true);
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
            self.bits.set_yaw_or_pitch_modified(true);
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
            self.bits.set_head_yaw_modified(true);
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
            self.bits.set_velocity_modified(true);
        }
    }

    /// Gets the value of the "on ground" flag.
    pub fn on_ground(&self) -> bool {
        self.bits.on_ground()
    }

    /// Sets the value of the "on ground" flag.
    pub fn set_on_ground(&mut self, on_ground: bool) {
        self.bits.set_on_ground(on_ground);
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
    /// [interact event]: crate::client::ClientEvent::InteractWithEntity
    pub fn hitbox(&self) -> Aabb<f64> {
        let dims = match &self.variants {
            TrackedData::Allay(_) => [0.6, 0.35, 0.6],
            TrackedData::ChestBoat(_) => [1.375, 0.5625, 1.375],
            TrackedData::Frog(_) => [0.5, 0.5, 0.5],
            TrackedData::Tadpole(_) => [0.4, 0.3, 0.4],
            TrackedData::Warden(_) => [0.9, 2.9, 0.9],
            TrackedData::AreaEffectCloud(e) => [
                e.get_radius() as f64 * 2.0,
                0.5,
                e.get_radius() as f64 * 2.0,
            ],
            TrackedData::ArmorStand(e) => {
                if e.get_marker() {
                    [0.0, 0.0, 0.0]
                } else if e.get_small() {
                    [0.5, 0.9875, 0.5]
                } else {
                    [0.5, 1.975, 0.5]
                }
            }
            TrackedData::Arrow(_) => [0.5, 0.5, 0.5],
            TrackedData::Axolotl(_) => [1.3, 0.6, 1.3],
            TrackedData::Bat(_) => [0.5, 0.9, 0.5],
            TrackedData::Bee(_) => [0.7, 0.6, 0.7], // TODO: baby size?
            TrackedData::Blaze(_) => [0.6, 1.8, 0.6],
            TrackedData::Boat(_) => [1.375, 0.5625, 1.375],
            TrackedData::Cat(_) => [0.6, 0.7, 0.6],
            TrackedData::CaveSpider(_) => [0.7, 0.5, 0.7],
            TrackedData::Chicken(_) => [0.4, 0.7, 0.4], // TODO: baby size?
            TrackedData::Cod(_) => [0.5, 0.3, 0.5],
            TrackedData::Cow(_) => [0.9, 1.4, 0.9], // TODO: baby size?
            TrackedData::Creeper(_) => [0.6, 1.7, 0.6],
            TrackedData::Dolphin(_) => [0.9, 0.6, 0.9],
            TrackedData::Donkey(_) => [1.5, 1.39648, 1.5], // TODO: baby size?
            TrackedData::DragonFireball(_) => [1.0, 1.0, 1.0],
            TrackedData::Drowned(_) => [0.6, 1.95, 0.6], // TODO: baby size?
            TrackedData::ElderGuardian(_) => [1.9975, 1.9975, 1.9975],
            TrackedData::EndCrystal(_) => [2.0, 2.0, 2.0],
            TrackedData::EnderDragon(_) => [16.0, 8.0, 16.0],
            TrackedData::Enderman(_) => [0.6, 2.9, 0.6],
            TrackedData::Endermite(_) => [0.4, 0.3, 0.4],
            TrackedData::Evoker(_) => [0.6, 1.95, 0.6],
            TrackedData::EvokerFangs(_) => [0.5, 0.8, 0.5],
            TrackedData::ExperienceOrb(_) => [0.5, 0.5, 0.5],
            TrackedData::EyeOfEnder(_) => [0.25, 0.25, 0.25],
            TrackedData::FallingBlock(_) => [0.98, 0.98, 0.98],
            TrackedData::FireworkRocket(_) => [0.25, 0.25, 0.25],
            TrackedData::Fox(_) => [0.6, 0.7, 0.6], // TODO: baby size?
            TrackedData::Ghast(_) => [4.0, 4.0, 4.0],
            TrackedData::Giant(_) => [3.6, 12.0, 3.6],
            TrackedData::GlowItemFrame(_) => todo!("account for rotation"),
            TrackedData::GlowSquid(_) => [0.8, 0.8, 0.8],
            TrackedData::Goat(_) => [1.3, 0.9, 1.3], // TODO: baby size?
            TrackedData::Guardian(_) => [0.85, 0.85, 0.85],
            TrackedData::Hoglin(_) => [1.39648, 1.4, 1.39648], // TODO: baby size?
            TrackedData::Horse(_) => [1.39648, 1.6, 1.39648],  // TODO: baby size?
            TrackedData::Husk(_) => [0.6, 1.95, 0.6],          // TODO: baby size?
            TrackedData::Illusioner(_) => [0.6, 1.95, 0.6],
            TrackedData::IronGolem(_) => [1.4, 2.7, 1.4],
            TrackedData::Item(_) => [0.25, 0.25, 0.25],
            TrackedData::ItemFrame(_) => todo!("account for rotation"),
            TrackedData::Fireball(_) => [1.0, 1.0, 1.0],
            TrackedData::LeashKnot(_) => [0.375, 0.5, 0.375],
            TrackedData::Lightning(_) => [0.0, 0.0, 0.0],
            TrackedData::Llama(_) => [0.9, 1.87, 0.9], // TODO: baby size?
            TrackedData::LlamaSpit(_) => [0.25, 0.25, 0.25],
            TrackedData::MagmaCube(e) => {
                let s = e.get_slime_size() as f64 * 0.51000005;
                [s, s, s]
            }
            TrackedData::Marker(_) => [0.0, 0.0, 0.0],
            TrackedData::Minecart(_) => [0.98, 0.7, 0.98],
            TrackedData::ChestMinecart(_) => [0.98, 0.7, 0.98],
            TrackedData::CommandBlockMinecart(_) => [0.98, 0.7, 0.98],
            TrackedData::FurnaceMinecart(_) => [0.98, 0.7, 0.98],
            TrackedData::HopperMinecart(_) => [0.98, 0.7, 0.98],
            TrackedData::SpawnerMinecart(_) => [0.98, 0.7, 0.98],
            TrackedData::TntMinecart(_) => [0.98, 0.7, 0.98],
            TrackedData::Mule(_) => [1.39648, 1.6, 1.39648], // TODO: baby size?
            TrackedData::Mooshroom(_) => [0.9, 1.4, 0.9],    // TODO: baby size?
            TrackedData::Ocelot(_) => [0.6, 0.7, 0.6],       // TODO: baby size?
            TrackedData::Painting(_) => todo!("account for rotation and type"),
            TrackedData::Panda(_) => [0.6, 0.7, 0.6], // TODO: baby size?
            TrackedData::Parrot(_) => [0.5, 0.9, 0.5],
            TrackedData::Phantom(_) => [0.9, 0.5, 0.9],
            TrackedData::Pig(_) => [0.9, 0.9, 0.9], // TODO: baby size?
            TrackedData::Piglin(_) => [0.6, 1.95, 0.6], // TODO: baby size?
            TrackedData::PiglinBrute(_) => [0.6, 1.95, 0.6],
            TrackedData::Pillager(_) => [0.6, 1.95, 0.6],
            TrackedData::PolarBear(_) => [1.4, 1.4, 1.4], // TODO: baby size?
            TrackedData::Tnt(_) => [0.98, 0.98, 0.98],
            TrackedData::Pufferfish(_) => [0.7, 0.7, 0.7],
            TrackedData::Rabbit(_) => [0.4, 0.5, 0.4], // TODO: baby size?
            TrackedData::Ravager(_) => [1.95, 2.2, 1.95],
            TrackedData::Salmon(_) => [0.7, 0.4, 0.7],
            TrackedData::Sheep(_) => [0.9, 1.3, 0.9], // TODO: baby size?
            TrackedData::Shulker(_) => [1.0, 1.0, 1.0], // TODO: how is height calculated?
            TrackedData::ShulkerBullet(_) => [0.3125, 0.3125, 0.3125],
            TrackedData::Silverfish(_) => [0.4, 0.3, 0.4],
            TrackedData::Skeleton(_) => [0.6, 1.99, 0.6],
            TrackedData::SkeletonHorse(_) => [1.39648, 1.6, 1.39648], // TODO: baby size?
            TrackedData::Slime(e) => {
                let s = 0.51000005 * e.get_slime_size() as f64;
                [s, s, s]
            }
            TrackedData::SmallFireball(_) => [0.3125, 0.3125, 0.3125],
            TrackedData::SnowGolem(_) => [0.7, 1.9, 0.7],
            TrackedData::Snowball(_) => [0.25, 0.25, 0.25],
            TrackedData::SpectralArrow(_) => [0.5, 0.5, 0.5],
            TrackedData::Spider(_) => [1.4, 0.9, 1.4],
            TrackedData::Squid(_) => [0.8, 0.8, 0.8],
            TrackedData::Stray(_) => [0.6, 1.99, 0.6],
            TrackedData::Strider(_) => [0.9, 1.7, 0.9], // TODO: baby size?
            TrackedData::Egg(_) => [0.25, 0.25, 0.25],
            TrackedData::EnderPearl(_) => [0.25, 0.25, 0.25],
            TrackedData::ExperienceBottle(_) => [0.25, 0.25, 0.25],
            TrackedData::Potion(_) => [0.25, 0.25, 0.25],
            TrackedData::Trident(_) => [0.5, 0.5, 0.5],
            TrackedData::TraderLlama(_) => [0.9, 1.87, 0.9],
            TrackedData::TropicalFish(_) => [0.5, 0.4, 0.5],
            TrackedData::Turtle(_) => [1.2, 0.4, 1.2], // TODO: baby size?
            TrackedData::Vex(_) => [0.4, 0.8, 0.4],
            TrackedData::Villager(_) => [0.6, 1.95, 0.6], // TODO: baby size?
            TrackedData::Vindicator(_) => [0.6, 1.95, 0.6],
            TrackedData::WanderingTrader(_) => [0.6, 1.95, 0.6],
            TrackedData::Witch(_) => [0.6, 1.95, 0.6],
            TrackedData::Wither(_) => [0.9, 3.5, 0.9],
            TrackedData::WitherSkeleton(_) => [0.7, 2.4, 0.7],
            TrackedData::WitherSkull(_) => [0.3125, 0.3125, 0.3125],
            TrackedData::Wolf(_) => [0.6, 0.85, 0.6], // TODO: baby size?
            TrackedData::Zoglin(_) => [1.39648, 1.4, 1.39648], // TODO: baby size?
            TrackedData::Zombie(_) => [0.6, 1.95, 0.6], // TODO: baby size?
            TrackedData::ZombieHorse(_) => [1.39648, 1.6, 1.39648], // TODO: baby size?
            TrackedData::ZombieVillager(_) => [0.6, 1.95, 0.6], // TODO: baby size?
            TrackedData::ZombifiedPiglin(_) => [0.6, 1.95, 0.6], // TODO: baby size?
            TrackedData::Player(_) => [0.6, 1.8, 0.6], // TODO: changes depending on the pose.
            TrackedData::FishingBobber(_) => [0.25, 0.25, 0.25],
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
    ) -> Option<SetEntityMetadata> {
        self.variants
            .initial_tracked_data()
            .map(|meta| SetEntityMetadata {
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
    ) -> Option<SetEntityMetadata> {
        self.variants
            .updated_tracked_data()
            .map(|meta| SetEntityMetadata {
                entity_id: VarInt(this_id.to_network_id()),
                metadata: RawBytes(meta),
            })
    }

    pub(crate) fn spawn_packet(&self, this_id: EntityId) -> Option<EntitySpawnPacket> {
        match &self.variants {
            TrackedData::Marker(_) => None,
            TrackedData::ExperienceOrb(_) => {
                Some(EntitySpawnPacket::ExperienceOrb(SpawnExperienceOrb {
                    entity_id: VarInt(this_id.to_network_id()),
                    position: self.new_position,
                    count: 0, // TODO
                }))
            }
            TrackedData::Player(_) => Some(EntitySpawnPacket::Player(SpawnPlayer {
                entity_id: VarInt(this_id.to_network_id()),
                player_uuid: self.uuid,
                position: self.new_position,
                yaw: ByteAngle::from_degrees(self.yaw),
                pitch: ByteAngle::from_degrees(self.pitch),
            })),
            _ => Some(EntitySpawnPacket::Entity(SpawnEntity {
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
    Entity(SpawnEntity),
    ExperienceOrb(SpawnExperienceOrb),
    Player(SpawnPlayer),
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

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use uuid::Uuid;

    use super::{Entities, EntityId, EntityKind};
    use crate::config::Config;
    use crate::server::Server;
    use crate::slab_versioned::Key;

    /// Created for the sole purpose of use during unit tests.
    struct MockConfig;
    impl Config for MockConfig {
        type ServerState = ();
        type ClientState = ();
        type EntityState = u8; // Just for identification purposes
        type WorldState = ();
        type ChunkState = ();
        type PlayerListState = ();

        fn max_connections(&self) -> usize {
            10
        }
        fn update(&self, _server: &mut Server<Self>) {}
    }

    #[test]
    fn entities_has_valid_new_state() {
        let mut entities: Entities<MockConfig> = Entities::new();
        let network_id: i32 = 8675309;
        let entity_id = EntityId(Key::new(
            202298,
            NonZeroU32::new(network_id as u32).expect("Value given should never be zero!"),
        ));
        let uuid = Uuid::from_bytes([2; 16]);
        assert!(entities.is_empty());
        assert!(entities.get(entity_id).is_none());
        assert!(entities.get_mut(entity_id).is_none());
        assert!(entities.get_with_uuid(uuid).is_none());
        assert!(entities.get_with_network_id(network_id).is_none());
    }

    #[test]
    fn entities_can_be_set_and_get() {
        let mut entities: Entities<MockConfig> = Entities::new();
        assert!(entities.is_empty());
        let (player_id, player_entity) = entities.insert(EntityKind::Player, 1);
        assert_eq!(player_entity.state, 1);
        assert_eq!(entities.get(player_id).unwrap().state, 1);
        let mut_player_entity = entities
            .get_mut(player_id)
            .expect("Failed to get mutable reference");
        mut_player_entity.state = 100;
        assert_eq!(entities.get(player_id).unwrap().state, 100);
        assert_eq!(entities.len(), 1);
    }

    #[test]
    fn entities_can_be_set_and_get_with_uuid() {
        let mut entities: Entities<MockConfig> = Entities::new();
        let uuid = Uuid::from_bytes([2; 16]);
        assert!(entities.is_empty());
        let (zombie_id, zombie_entity) = entities
            .insert_with_uuid(EntityKind::Zombie, uuid, 1)
            .expect("Unexpected Uuid collision when inserting to an empty collection");
        assert_eq!(zombie_entity.state, 1);
        let maybe_zombie = entities
            .get_with_uuid(uuid)
            .expect("Uuid lookup failed on item already added to this collection");
        assert_eq!(zombie_id, maybe_zombie);
        assert_eq!(entities.len(), 1);
    }

    #[test]
    fn entities_can_be_set_and_get_with_network_id() {
        let mut entities: Entities<MockConfig> = Entities::new();
        assert!(entities.is_empty());
        let (boat_id, boat_entity) = entities.insert(EntityKind::Boat, 12);
        assert_eq!(boat_entity.state, 12);
        let (cat_id, cat_entity) = entities.insert(EntityKind::Cat, 75);
        assert_eq!(cat_entity.state, 75);
        let maybe_boat_id = entities
            .get_with_network_id(boat_id.0.version.get() as i32)
            .expect("Network id lookup failed on item already added to this collection");
        let maybe_boat = entities
            .get(maybe_boat_id)
            .expect("Failed to look up item already added to collection");
        assert_eq!(maybe_boat.state, 12);
        let maybe_cat_id = entities
            .get_with_network_id(cat_id.0.version.get() as i32)
            .expect("Network id lookup failed on item already added to this collection");
        let maybe_cat = entities
            .get(maybe_cat_id)
            .expect("Failed to look up item already added to collection");
        assert_eq!(maybe_cat.state, 75);
        assert_eq!(entities.len(), 2);
    }

    #[test]
    fn entities_can_be_removed() {
        let mut entities: Entities<MockConfig> = Entities::new();
        assert!(entities.is_empty());
        let (player_id, _) = entities.insert(EntityKind::Player, 1);
        let player_state = entities
            .remove(player_id)
            .expect("Failed to remove an item from the collection");
        assert_eq!(player_state, 1);
    }

    #[test]
    fn entities_can_be_retained() {
        let mut entities: Entities<MockConfig> = Entities::new();
        assert!(entities.is_empty());
        let (blaze_id, _) = entities.insert(EntityKind::Blaze, 10);
        let (fox_id, _) = entities.insert(EntityKind::Fox, 110);
        let (turtle_id, _) = entities.insert(EntityKind::Turtle, 20);
        let (goat_id, _) = entities.insert(EntityKind::Goat, 120);
        let (horse_id, _) = entities.insert(EntityKind::Horse, 30);
        assert_eq!(entities.len(), 5);
        entities.retain(|_id, entity| entity.state > 100);
        assert_eq!(entities.len(), 2);
        assert!(entities.get(fox_id).is_some());
        assert!(entities.get(goat_id).is_some());
        assert!(entities.get(blaze_id).is_none());
        assert!(entities.get(turtle_id).is_none());
        assert!(entities.get(horse_id).is_none());
    }
}
